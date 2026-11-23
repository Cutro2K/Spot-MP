use std::sync::Arc;
use std::sync::RwLock;
use reqwest::{Client};

use crate::{domain::spot_mp_errors::{SpotMpError, Status}, services::spotify_api::client::request_builder::SpotifyRequestBuilder};

pub const DEFAULT_ACCOUNTS_BASE_URL: &str = "https://accounts.spotify.com";
pub const DEFAULT_API_BASE_URL: &str = "https://api.spotify.com";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpotifyEndpoints {
    accounts_base_url: String,
    api_base_url: String,
}

impl Default for SpotifyEndpoints {
    fn default() -> Self {
        Self::new(DEFAULT_ACCOUNTS_BASE_URL, DEFAULT_API_BASE_URL)
    }
}

impl SpotifyEndpoints {
    pub fn new(
        accounts_base_url: impl Into<String>,
        api_base_url: impl Into<String>,
    ) -> Self {
        Self {
            accounts_base_url: accounts_base_url.into().trim_end_matches('/').to_string(),
            api_base_url: api_base_url.into().trim_end_matches('/').to_string(),
        }
    }

    pub fn accounts_url(&self, path: &str) -> String {
        format!("{}/{}", self.accounts_base_url, path.trim_start_matches('/'))
    }

    pub fn api_url(&self, path: &str) -> String {
        format!("{}/{}", self.api_base_url, path.trim_start_matches('/'))
    }
}

// Shared Web API session and token state
#[derive(Default)]
pub struct TokenState {
    pub client_id: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub session_epoch: u64,
}

#[derive(Clone)]
pub struct SpotifyHttpClient {
    client: Client,
    tokens: Arc<RwLock<TokenState>>,
    endpoints: SpotifyEndpoints,
}

impl SpotifyHttpClient {
    pub fn new(tokens: Arc<RwLock<TokenState>>) -> Self {
        Self::with_endpoints(tokens, SpotifyEndpoints::default())
    }

    pub fn with_endpoints(
        tokens: Arc<RwLock<TokenState>>,
        endpoints: SpotifyEndpoints,
    ) -> Self {
        Self {
            client: Client::new(),
            tokens,
            endpoints,
        }
    }

    pub fn accounts_url(&self, path: &str) -> String {
        self.endpoints.accounts_url(path)
    }

    pub fn api_url(&self, path: &str) -> String {
        self.endpoints.api_url(path)
    }

    // Centralized error mapping function for http results.
    pub fn map_error_status(status_code: u16, code_text: String) -> SpotMpError {
        match status_code {
            400 => SpotMpError::SpotifyErrorStatus(Status::BadRequest(code_text)),
            401 => SpotMpError::SpotifyErrorStatus(Status::Unauthorized(code_text)),
            429 => SpotMpError::SpotifyErrorStatus(Status::TooManyRequest(code_text)),
            _ => SpotMpError::SpotifyErrorStatus(Status::Unknown(status_code, code_text)),
        }
    }

    // Request an access token safety
    pub fn require_token(&self) -> Result<String, SpotMpError> {
        let guard = self.tokens.read().unwrap();
        guard.access_token.clone().ok_or_else(|| {
            SpotMpError::SpotifyErrorStatus(Status::Unauthorized(
                "The access token is missing!".to_string(),
            ))
        })
    }
    pub fn require_refresh(&self) -> Result<String, SpotMpError> {
        self.tokens
            .read()
            .unwrap()
            .refresh_token
            .clone()
            .ok_or_else(|| {
                SpotMpError::SessionExpired(String::from(
                    "Tried to renew token before authentication (refresh_token is missing)",
                ))
            })
    }

    pub fn require_client_id(&self) -> Result<String, SpotMpError> {
        self.tokens
            .read()
            .unwrap()
            .client_id
            .clone()
            .ok_or_else(|| {
                SpotMpError::SessionExpired(String::from(
                    "Cannot authenticate or renew token (client_id is missing)",
                ))
            })
    }

    pub fn snapshot_refresh_credentials(&self) -> Result<(u64, String, String), SpotMpError> {
        let guard = self.tokens.read().unwrap();
        let client_id = guard.client_id.clone().ok_or_else(|| {
            SpotMpError::SessionExpired(String::from(
                "Cannot authenticate or renew token (client_id is missing)",
            ))
        })?;
        let refresh_token = guard.refresh_token.clone().ok_or_else(|| {
            SpotMpError::SessionExpired(String::from(
                "Tried to renew token before authentication (refresh_token is missing)",
            ))
        })?;
        Ok((guard.session_epoch, client_id, refresh_token))
    }

    // --- Entry points for builder pattern ---
    pub fn get(&self, url: &str) -> SpotifyRequestBuilder<'_> {
        SpotifyRequestBuilder::new(self, self.client.get(url))
    }
    pub fn post(&self, url: &str) -> SpotifyRequestBuilder<'_> {
        SpotifyRequestBuilder::new(self, self.client.post(url))
    }
    pub fn delete(&self, url: &str) -> SpotifyRequestBuilder<'_> {
        SpotifyRequestBuilder::new(self, self.client.delete(url))
    }

    pub fn set_session(
        &self,
        client_id: Option<String>,
        access_token: Option<String>,
        refresh_token: Option<String>,
    ) {
        let mut guard = self.tokens.write().unwrap();
        guard.client_id = client_id;
        guard.access_token = access_token;
        guard.refresh_token = refresh_token;
        guard.session_epoch = guard.session_epoch.wrapping_add(1);
    }

    pub fn clear_web_api_tokens(&self) {
        let mut guard = self.tokens.write().unwrap();
        guard.access_token = None;
        guard.refresh_token = None;
        guard.session_epoch = guard.session_epoch.wrapping_add(1);
    }

    pub fn update_tokens_if_epoch(
        &self,
        expected_epoch: u64,
        access_token: String,
        refresh_token: String,
    ) -> bool {
        let mut guard = self.tokens.write().unwrap();
        if guard.session_epoch != expected_epoch {
            return false;
        }
        guard.access_token = Some(access_token);
        guard.refresh_token = Some(refresh_token);
        true
    }

    pub fn client_id(&self) -> Option<String> {
        self.tokens.read().unwrap().client_id.clone()
    }

    pub fn session_epoch(&self) -> u64 {
        self.tokens.read().unwrap().session_epoch
    }

    pub fn access_token(&self) -> Option<String> {
        self.tokens.read().unwrap().access_token.clone()
    }

    pub fn refresh_token(&self) -> Option<String> {
        self.tokens.read().unwrap().refresh_token.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_http_client() -> SpotifyHttpClient {
        SpotifyHttpClient::new(Arc::new(RwLock::new(TokenState::default())))
    }

    #[test]
    fn set_session_stores_client_id_tokens_and_increments_epoch() {
        let http = new_http_client();
        assert_eq!(http.session_epoch(), 0);
        assert_eq!(http.client_id(), None);
        assert!(http.require_client_id().is_err());

        http.set_session(
            Some("client-123".to_string()),
            Some("access-1".to_string()),
            Some("refresh-1".to_string()),
        );

        assert_eq!(http.session_epoch(), 1);
        assert_eq!(http.client_id().as_deref(), Some("client-123"));
        assert_eq!(http.require_client_id().unwrap(), "client-123");
        assert_eq!(http.access_token().as_deref(), Some("access-1"));
        assert_eq!(http.refresh_token().as_deref(), Some("refresh-1"));
    }

    #[test]
    fn clear_web_api_tokens_preserves_client_id_and_increments_epoch() {
        let http = new_http_client();
        http.set_session(
            Some("client-123".to_string()),
            Some("access-1".to_string()),
            Some("refresh-1".to_string()),
        );
        let epoch_after_set = http.session_epoch();

        http.clear_web_api_tokens();

        assert_eq!(http.access_token(), None);
        assert_eq!(http.refresh_token(), None);
        assert_eq!(http.client_id().as_deref(), Some("client-123"));
        assert!(http.session_epoch() > epoch_after_set);

        let epoch_after_clear = http.session_epoch();
        http.set_session(None, None, None);
        assert_eq!(http.client_id(), None);
        assert_eq!(http.access_token(), None);
        assert_eq!(http.refresh_token(), None);
        assert!(http.session_epoch() > epoch_after_clear);
    }

    #[test]
    fn update_tokens_if_epoch_only_writes_when_epoch_matches() {
        let http = new_http_client();
        http.set_session(
            Some("client-123".to_string()),
            Some("access-1".to_string()),
            Some("refresh-1".to_string()),
        );
        let epoch_v1 = http.session_epoch();

        // Matching epoch updates tokens and keeps client_id intact.
        let updated = http.update_tokens_if_epoch(
            epoch_v1,
            "access-2".to_string(),
            "refresh-2".to_string(),
        );
        assert!(updated);
        assert_eq!(http.access_token().as_deref(), Some("access-2"));
        assert_eq!(http.refresh_token().as_deref(), Some("refresh-2"));
        assert_eq!(http.client_id().as_deref(), Some("client-123"));

        // Logout / clear advances epoch; stale epoch write must be rejected.
        http.set_session(None, None, None);
        let stale_updated = http.update_tokens_if_epoch(
            epoch_v1,
            "access-stale".to_string(),
            "refresh-stale".to_string(),
        );
        assert!(!stale_updated);
        assert_eq!(http.access_token(), None);
        assert_eq!(http.refresh_token(), None);
        assert_eq!(http.client_id(), None);
    }

    #[test]
    fn snapshot_refresh_credentials_reads_epoch_client_id_and_refresh_token_atomically() {
        let http = new_http_client();
        assert!(matches!(
            http.snapshot_refresh_credentials(),
            Err(SpotMpError::SessionExpired(_))
        ));

        http.set_session(
            Some("client-xyz".to_string()),
            Some("access-1".to_string()),
            Some("refresh-1".to_string()),
        );
        let (epoch, client_id, refresh_token) = http.snapshot_refresh_credentials().unwrap();
        assert_eq!(epoch, http.session_epoch());
        assert_eq!(client_id, "client-xyz");
        assert_eq!(refresh_token, "refresh-1");

        http.clear_web_api_tokens();
        assert!(matches!(
            http.snapshot_refresh_credentials(),
            Err(SpotMpError::SessionExpired(_))
        ));
    }

    #[test]
    fn endpoints_default_to_spotify_production_urls_and_support_custom_bases() {
        let default_http = new_http_client();
        assert_eq!(
            default_http.accounts_url("/authorize"),
            "https://accounts.spotify.com/authorize"
        );
        assert_eq!(
            default_http.accounts_url("/api/token"),
            "https://accounts.spotify.com/api/token"
        );
        assert_eq!(
            default_http.api_url("/v1/me"),
            "https://api.spotify.com/v1/me"
        );

        let custom_endpoints = SpotifyEndpoints::new(
            "http://127.0.0.1:4100/",
            "http://127.0.0.1:4200/",
        );
        let custom_http = SpotifyHttpClient::with_endpoints(
            Arc::new(RwLock::new(TokenState::default())),
            custom_endpoints,
        );
        assert_eq!(
            custom_http.accounts_url("/api/token"),
            "http://127.0.0.1:4100/api/token"
        );
        assert_eq!(
            custom_http.api_url("v1/me/albums?limit=10"),
            "http://127.0.0.1:4200/v1/me/albums?limit=10"
        );
    }
}