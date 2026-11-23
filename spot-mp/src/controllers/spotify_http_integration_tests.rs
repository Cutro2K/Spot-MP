use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::controllers::test_doubles::{TestHubHarness, seeded_session};
use crate::domain::auth_models::WebApiData;
use crate::domain::spot_mp_errors::SpotMpError;
use crate::domain::track_collection::TrackCollection;
use crate::services::spotify_api::client::http_client::SpotifyEndpoints;
use crate::services::spotify_api::spotify_api_client::SpotifyApiClient;
use crate::state_store::app::action::{Action, SpotifyAction};

#[derive(Clone, Debug)]
struct RecordedRequest {
    method: String,
    target: String,
    headers: HashMap<String, String>,
    body: String,
}

impl RecordedRequest {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    fn path(&self) -> &str {
        self.target.split('?').next().unwrap_or(&self.target)
    }

    fn form_params(&self) -> HashMap<String, String> {
        reqwest::Url::parse(&format!("http://stub/?{}", self.body))
            .map(|url| url.query_pairs().into_owned().collect())
            .unwrap_or_default()
    }
}

struct StubResponse {
    status: u16,
    body: String,
}

impl StubResponse {
    fn json(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            body: body.into(),
        }
    }

    fn reason(&self) -> &'static str {
        match self.status {
            200 => "OK",
            400 => "Bad Request",
            401 => "Unauthorized",
            404 => "Not Found",
            500 => "Internal Server Error",
            _ => "Status",
        }
    }
}

struct StubHttpServer {
    base_url: String,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
    task: JoinHandle<()>,
}

impl StubHttpServer {
    async fn start<F>(handler: F) -> Self
    where
        F: Fn(&RecordedRequest) -> StubResponse + Send + Sync + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind local stub http server");
        let addr = listener.local_addr().expect("stub server local addr");
        let base_url = format!("http://{addr}");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&requests);
        let handler = Arc::new(handler);

        let task = tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let recorded = Arc::clone(&recorded);
                let handler = Arc::clone(&handler);
                tokio::spawn(async move {
                    if let Some(req) = read_http_request(&mut stream).await {
                        recorded.lock().unwrap().push(req.clone());
                        let resp = handler(&req);
                        let raw = format!(
                            "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            resp.status,
                            resp.reason(),
                            resp.body.len(),
                            resp.body
                        );
                        let _ = stream.write_all(raw.as_bytes()).await;
                        let _ = stream.shutdown().await;
                    }
                });
            }
        });

        Self {
            base_url,
            requests,
            task,
        }
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for StubHttpServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn read_http_request(stream: &mut TcpStream) -> Option<RecordedRequest> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];

    let header_end = loop {
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos;
        }
        let n = stream.read(&mut chunk).await.ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
    };

    let header_text = String::from_utf8_lossy(&buf[..header_end]);
    let mut lines = header_text.split("\r\n");
    let request_line = lines.next()?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?.to_string();

    let mut headers = HashMap::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    let content_length = headers
        .get("content-length")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);

    let body_start = header_end + 4;
    while buf.len() < body_start + content_length {
        let n = stream.read(&mut chunk).await.ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }

    let body_end = (body_start + content_length).min(buf.len());
    let body = String::from_utf8_lossy(&buf[body_start..body_end]).to_string();

    Some(RecordedRequest {
        method,
        target,
        headers,
        body,
    })
}

const SAMPLE_ALBUMS_JSON: &str = r#"{
    "next": null,
    "items": [
        {
            "album": {
                "id": "alb-1",
                "name": "Discovery",
                "images": [{"url": "https://img.test/discovery.jpg"}],
                "artists": [{"name": "Daft Punk"}],
                "release_date": "2001-03-12"
            }
        }
    ]
}"#;

const SAMPLE_ALBUM_TRACKS_JSON: &str = r#"{
    "next": null,
    "items": [
        {
            "name": "One More Time",
            "duration_ms": 320000,
            "id": "trk-1",
            "artists": [{"name": "Daft Punk"}]
        }
    ]
}"#;

const SAMPLE_USER_PROFILE_JSON: &str = r#"{"id":"user-42"}"#;

const SAMPLE_PLAYLISTS_JSON: &str = r#"{
    "next": null,
    "items": [
        {
            "collaborative": false,
            "id": "pl-1",
            "images": [{"url": "https://img.test/playlist.jpg"}],
            "owner": {"id": "user-42"},
            "name": "Favorites"
        }
    ]
}"#;

const SAMPLE_PLAYLIST_TRACKS_JSON: &str = r#"{
    "next": null,
    "items": [
        {
            "track": {
                "name": "One More Time",
                "duration_ms": 320000,
                "id": "trk-1",
                "artists": [{"name": "Daft Punk"}],
                "album": {
                    "id": "alb-1",
                    "name": "Discovery",
                    "images": [{"url": "https://img.test/discovery.jpg"}],
                    "artists": [{"name": "Daft Punk"}],
                    "release_date": "2001-03-12"
                }
            }
        }
    ]
}"#;

fn handle_token_refresh(req: &RecordedRequest) -> StubResponse {
    match req.form_params().get("refresh_token").map(String::as_str) {
        Some("refresh-v1") => StubResponse::json(
            200,
            r#"{"access_token":"access-v2","token_type":"Bearer","expires_in":3600,"refresh_token":"refresh-v2"}"#,
        ),
        _ => StubResponse::json(
            400,
            r#"{"error":"invalid_grant","error_description":"Refresh token revoked"}"#,
        ),
    }
}

fn handle_authenticated_api(req: &RecordedRequest, valid_access: &str) -> StubResponse {
    let expected = format!("Bearer {valid_access}");
    if req.header("authorization") != Some(expected.as_str()) {
        return StubResponse::json(
            401,
            r#"{"error":{"status":401,"message":"The access token expired"}}"#,
        );
    }
    match (req.method.as_str(), req.path()) {
        ("GET", "/v1/me") => StubResponse::json(200, SAMPLE_USER_PROFILE_JSON),
        ("GET", "/v1/me/playlists") => StubResponse::json(200, SAMPLE_PLAYLISTS_JSON),
        ("GET", "/v1/playlists/pl-1/items") => StubResponse::json(200, SAMPLE_PLAYLIST_TRACKS_JSON),
        ("GET", "/v1/me/albums") => StubResponse::json(200, SAMPLE_ALBUMS_JSON),
        ("GET", "/v1/albums/alb-1/tracks") => StubResponse::json(200, SAMPLE_ALBUM_TRACKS_JSON),
        _ => StubResponse::json(404, r#"{"error":"not_found"}"#),
    }
}

#[tokio::test]
async fn spotify_api_client_rotates_refresh_token_on_401_and_expires_on_invalid_grant() {
    let valid_access = Arc::new(Mutex::new("access-v2".to_string()));
    let valid_access_handler = Arc::clone(&valid_access);

    // Distinct Accounts and Web API stub servers confirm both base URLs route independently.
    let accounts_server = StubHttpServer::start(|req| match (req.method.as_str(), req.path()) {
        ("POST", "/api/token") => handle_token_refresh(req),
        _ => StubResponse::json(404, r#"{"error":"not_found"}"#),
    })
    .await;

    let api_server = StubHttpServer::start(move |req| {
        let valid = valid_access_handler.lock().unwrap().clone();
        handle_authenticated_api(req, &valid)
    })
    .await;

    let endpoints = SpotifyEndpoints::new(accounts_server.base_url(), api_server.base_url());
    let mut client = SpotifyApiClient::with_endpoints(endpoints);
    client.restore_session(
        "client-123".to_string(),
        "access-v1".to_string(),
        "refresh-v1".to_string(),
    );

    // 1. Expired access token triggers 401 -> POST /api/token rotates refresh token -> retry succeeds
    let albums = client
        .import_liked_albums()
        .await
        .expect("import_liked_albums should succeed after reactive refresh");
    assert_eq!(albums.len(), 1);
    assert_eq!(albums[0].name(), "Discovery");
    assert_eq!(client.access_token().as_deref(), Some("access-v2"));
    assert_eq!(client.refresh_token().as_deref(), Some("refresh-v2"));
    assert_eq!(client.client_id().as_deref(), Some("client-123"));

    // Verify wire traffic on Accounts server: form-encoded refresh_token grant without Bearer header
    let account_reqs = accounts_server.requests();
    assert_eq!(account_reqs.len(), 1);
    assert_eq!(account_reqs[0].method, "POST");
    assert_eq!(account_reqs[0].path(), "/api/token");
    assert_eq!(account_reqs[0].header("authorization"), None);
    let form = account_reqs[0].form_params();
    assert_eq!(
        form.get("grant_type").map(String::as_str),
        Some("refresh_token")
    );
    assert_eq!(
        form.get("refresh_token").map(String::as_str),
        Some("refresh-v1")
    );
    assert_eq!(form.get("client_id").map(String::as_str), Some("client-123"));

    // Verify wire traffic on Web API server: initial 401 with old token, then retry with access-v2
    let api_reqs = api_server.requests();
    assert_eq!(api_reqs[0].path(), "/v1/me/albums");
    assert_eq!(api_reqs[0].header("authorization"), Some("Bearer access-v1"));
    assert_eq!(api_reqs[1].path(), "/v1/me/albums");
    assert_eq!(api_reqs[1].header("authorization"), Some("Bearer access-v2"));

    // 2. Expire access-v2 on the stub and reject refresh-v2 with 400 invalid_grant -> SessionExpired
    *valid_access.lock().unwrap() = "access-v3-unreachable".to_string();
    let err = client
        .import_liked_albums()
        .await
        .expect_err("revoked refresh token must return SessionExpired");
    assert!(
        matches!(err, SpotMpError::SessionExpired(_)),
        "expected SpotMpError::SessionExpired, got: {err:?}"
    );
    assert_eq!(client.access_token(), None);
    assert_eq!(client.refresh_token(), None);
    assert_eq!(client.client_id().as_deref(), Some("client-123"));
}

#[tokio::test]
async fn hub_end_to_end_401_refresh_rotates_tokens_and_persists_to_session_json() {
    let server = StubHttpServer::start(|req| match (req.method.as_str(), req.path()) {
        ("POST", "/api/token") => handle_token_refresh(req),
        _ => handle_authenticated_api(req, "access-v2"),
    })
    .await;

    let initial = seeded_session();
    let endpoints = SpotifyEndpoints::new(server.base_url(), server.base_url());
    let mut harness = TestHubHarness::with_client(
        "http-rotate-persist",
        initial.clone(),
        SpotifyApiClient::with_endpoints(endpoints),
    );

    // FetchPlaylists exercises /v1/me (UserService) + /v1/me/playlists + /v1/playlists/{id}/items
    harness
        .hub
        .handle_spotify_action(SpotifyAction::FetchPlaylists)
        .await;

    let action = harness.dispatched_rx.try_recv().expect("dispatched action");
    match action {
        Action::Spotify(SpotifyAction::FetchedPlaylists(playlists)) => {
            assert_eq!(playlists.len(), 1);
            assert_eq!(playlists[0].name(), "Favorites");
        }
        other => panic!("expected FetchedPlaylists, got: {other:?}"),
    }

    // Verify wire traffic: initial /v1/me returned 401, /api/token rotated refresh-v1 -> refresh-v2,
    // and retried /v1/me + playlists requests carried Bearer access-v2.
    let recorded = server.requests();
    assert_eq!(recorded[0].path(), "/v1/me");
    assert_eq!(recorded[0].header("authorization"), Some("Bearer access-v1"));
    assert_eq!(recorded[1].method, "POST");
    assert_eq!(recorded[1].path(), "/api/token");
    assert_eq!(recorded[1].header("authorization"), None);
    let form = recorded[1].form_params();
    assert_eq!(
        form.get("grant_type").map(String::as_str),
        Some("refresh_token")
    );
    assert_eq!(
        form.get("refresh_token").map(String::as_str),
        Some("refresh-v1")
    );
    assert_eq!(form.get("client_id").map(String::as_str), Some("client-123"));
    assert_eq!(recorded[2].path(), "/v1/me");
    assert_eq!(recorded[2].header("authorization"), Some("Bearer access-v2"));

    // Rotated access + refresh tokens are persisted to session.json alongside client_id & librespot.
    let persisted = harness.read_session_from_disk();
    assert_eq!(persisted.client_id, initial.client_id);
    assert_eq!(persisted.librespot, initial.librespot);
    assert_eq!(
        persisted.web_api,
        Some(WebApiData {
            access_token: "access-v2".to_string(),
            refresh_token: "refresh-v2".to_string(),
        })
    );

    // Subsequent call uses rotated access-v2 in memory with zero extra /api/token calls.
    harness
        .hub
        .handle_spotify_action(SpotifyAction::FetchAlbums)
        .await;
    let second_action = harness
        .dispatched_rx
        .try_recv()
        .expect("second dispatched action");
    assert!(matches!(
        second_action,
        Action::Spotify(SpotifyAction::FetchedAlbums(_))
    ));

    let token_calls = server
        .requests()
        .into_iter()
        .filter(|r| r.method == "POST" && r.path() == "/api/token")
        .count();
    assert_eq!(
        token_calls, 1,
        "rotated access token must remain cached in memory for subsequent requests"
    );
}

#[tokio::test]
async fn hub_end_to_end_invalid_grant_returns_session_expired_and_preserves_client_id_and_librespot()
{
    let valid_access = Arc::new(Mutex::new("access-v2".to_string()));
    let valid_access_handler = Arc::clone(&valid_access);

    let server = StubHttpServer::start(move |req| match (req.method.as_str(), req.path()) {
        ("POST", "/api/token") => handle_token_refresh(req),
        _ => {
            let valid = valid_access_handler.lock().unwrap().clone();
            handle_authenticated_api(req, &valid)
        }
    })
    .await;

    let initial = seeded_session();
    let endpoints = SpotifyEndpoints::new(server.base_url(), server.base_url());
    let mut harness = TestHubHarness::with_client(
        "http-invalid-grant",
        initial.clone(),
        SpotifyApiClient::with_endpoints(endpoints),
    );

    // 1. First action succeeds after rotating refresh-v1 -> refresh-v2.
    harness
        .hub
        .handle_spotify_action(SpotifyAction::FetchAlbums)
        .await;
    assert!(matches!(
        harness.dispatched_rx.try_recv().expect("first action"),
        Action::Spotify(SpotifyAction::FetchedAlbums(_))
    ));

    // 2. Expire access-v2; stub rejects refresh-v2 with 400 invalid_grant.
    *valid_access.lock().unwrap() = "access-v3-unreachable".to_string();
    harness
        .hub
        .handle_spotify_action(SpotifyAction::FetchAlbums)
        .await;

    let expired_action = harness
        .dispatched_rx
        .try_recv()
        .expect("session expired action");
    assert!(
        matches!(expired_action, Action::Spotify(SpotifyAction::SessionExpired)),
        "expected SessionExpired action, got: {expired_action:?}"
    );

    // Confirm the failed refresh actually sent the rotated refresh-v2 token over HTTP.
    let token_reqs: Vec<_> = server
        .requests()
        .into_iter()
        .filter(|r| r.method == "POST" && r.path() == "/api/token")
        .collect();
    assert_eq!(token_reqs.len(), 2);
    assert_eq!(
        token_reqs[1]
            .form_params()
            .get("refresh_token")
            .map(String::as_str),
        Some("refresh-v2")
    );

    // Only web_api is cleared in session.json; client_id and librespot credentials remain intact.
    let persisted = harness.read_session_from_disk();
    assert_eq!(persisted.web_api, None);
    assert_eq!(persisted.client_id, initial.client_id);
    assert_eq!(persisted.librespot, initial.librespot);

    // Librespot was not logged out.
    assert!(
        matches!(
            harness.audio_msg_rx.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ),
        "librespot must remain active after Web API SessionExpired"
    );
}

#[tokio::test]
async fn hub_end_to_end_rotated_tokens_persist_when_retried_api_call_returns_500() {
    let server = StubHttpServer::start(|req| match (req.method.as_str(), req.path()) {
        ("POST", "/api/token") => handle_token_refresh(req),
        ("GET", "/v1/search") => {
            if req.header("authorization") == Some("Bearer access-v2") {
                StubResponse::json(
                    500,
                    r#"{"error":{"status":500,"message":"upstream failure"}}"#,
                )
            } else {
                StubResponse::json(401, r#"{"error":{"status":401}}"#)
            }
        }
        _ => StubResponse::json(404, r#"{"error":"not_found"}"#),
    })
    .await;

    let initial = seeded_session();
    let endpoints = SpotifyEndpoints::new(server.base_url(), server.base_url());
    let mut harness = TestHubHarness::with_client(
        "http-rotate-then-500",
        initial.clone(),
        SpotifyApiClient::with_endpoints(endpoints),
    );

    harness
        .hub
        .handle_spotify_action(SpotifyAction::SearchQuery("daft punk".to_string()))
        .await;

    let err_action = harness
        .dispatched_rx
        .try_recv()
        .expect("dispatched error action");
    assert!(
        matches!(err_action, Action::UnexpectedErrorHappened(_)),
        "expected UnexpectedErrorHappened on retried 500, got: {err_action:?}"
    );

    // Confirm initial 401 -> /api/token rotation -> retried /v1/search with access-v2.
    let search_reqs: Vec<_> = server
        .requests()
        .into_iter()
        .filter(|r| r.method == "GET" && r.path() == "/v1/search")
        .collect();
    assert_eq!(search_reqs.len(), 2);
    assert_eq!(
        search_reqs[0].header("authorization"),
        Some("Bearer access-v1")
    );
    assert_eq!(
        search_reqs[1].header("authorization"),
        Some("Bearer access-v2")
    );

    // Rotated tokens MUST still be persisted to session.json despite the retried 500 error.
    let persisted = harness.read_session_from_disk();
    assert_eq!(persisted.client_id, initial.client_id);
    assert_eq!(persisted.librespot, initial.librespot);
    assert_eq!(
        persisted.web_api,
        Some(WebApiData {
            access_token: "access-v2".to_string(),
            refresh_token: "refresh-v2".to_string(),
        })
    );
}
