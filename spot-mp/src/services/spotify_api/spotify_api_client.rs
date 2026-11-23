use std::sync::{Arc, RwLock};
use futures::try_join;
use tokio::sync::Mutex;

use crate::{
    domain::{
        album::Album,
        playlist::{Playlist, SpotifySearchResponse},
        song::Song,
        spot_mp_errors::{SpotMpError, Status},
    },
    services::spotify_api::{
        auth::{auth_service::AuthService, oauth_config::OAuthConfig},
        client::http_client::{SpotifyHttpClient, TokenState},
        services::{
            album_service::AlbumService, playlist_service::PlaylistService,
            search_service::SearchService, user_service::UserService,
        },
        traits::{AlbumManager, AuthManager, PlaylistManager, SearchManager, UserManager},
    },
};

pub struct SpotifyApiClient<
    A: AuthManager = AuthService,
    U: UserManager = UserService,
    P: PlaylistManager = PlaylistService,
    Al: AlbumManager = AlbumService,
    S: SearchManager = SearchService,
> {
    auth: A,
    user: U,
    playlist: P,
    album: Al,
    search: S,
    refresh_lock: Mutex<()>,
}

impl Default for SpotifyApiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl SpotifyApiClient {
    pub fn new() -> Self {
        let tokens = Arc::new(RwLock::new(TokenState::default()));
        let http = SpotifyHttpClient::new(tokens);

        Self {
            auth: AuthService::new(http.clone(), OAuthConfig::default()),
            user: UserService::new(http.clone()),
            playlist: PlaylistService::new(http.clone()),
            album: AlbumService::new(http.clone()),
            search: SearchService::new(http),
            refresh_lock: Mutex::new(()),
        }
    }
}

impl<A, U, P, Al, S> SpotifyApiClient<A, U, P, Al, S>
where
    A: AuthManager,
    U: UserManager,
    P: PlaylistManager,
    Al: AlbumManager,
    S: SearchManager,
{
    pub fn with_services(auth: A, user: U, playlist: P, album: Al, search: S) -> Self {
        Self {
            auth,
            user,
            playlist,
            album,
            search,
            refresh_lock: Mutex::new(()),
        }
    }

    fn is_terminal_refresh_error(err: &SpotMpError) -> bool {
        matches!(
            err,
            SpotMpError::SpotifyErrorStatus(Status::BadRequest(_) | Status::Unauthorized(_))
                | SpotMpError::SessionExpired(_)
        )
    }

    async fn execute_with_refresh<T, F, Fut>(&self, mut op: F) -> Result<T, SpotMpError>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, SpotMpError>>,
    {
        let token_before = self.auth.access_token();
        let epoch_before = self.auth.session_epoch();
        match op().await {
            Ok(result) => Ok(result),
            Err(SpotMpError::SpotifyErrorStatus(Status::Unauthorized(_))) => {
                {
                    let _guard = self.refresh_lock.lock().await;
                    if self.auth.session_epoch() != epoch_before {
                        return Err(SpotMpError::SessionExpired(
                            "Session changed while waiting to refresh token".to_string(),
                        ));
                    }
                    let current = self.auth.access_token();
                    let already_refreshed = current != token_before && current.is_some();
                    if !already_refreshed {
                        let refresh_result = self.auth.renew_token().await;
                        if self.auth.session_epoch() != epoch_before {
                            return Err(SpotMpError::SessionExpired(
                                "Session changed during token refresh".to_string(),
                            ));
                        }
                        if let Err(refresh_err) = refresh_result {
                            if Self::is_terminal_refresh_error(&refresh_err) {
                                self.auth.clear_tokens();
                                self.user.clear_user_id();
                                return Err(match refresh_err {
                                    SpotMpError::SessionExpired(_) => refresh_err,
                                    other => SpotMpError::SessionExpired(other.to_string()),
                                });
                            }
                            return Err(refresh_err);
                        }
                    }
                }
                op().await
            }
            Err(err) => Err(err),
        }
    }

    // --- AuthManager ---
    pub async fn login(&mut self, client_id: &str) -> Result<(), SpotMpError> {
        self.user.clear_user_id();
        self.auth.login(client_id).await?;
        let _ = self.user.fetch_current_user_id().await;
        Ok(())
    }

    pub fn logout(&self) {
        self.auth.logout();
        self.user.clear_user_id();
    }

    pub fn restore_session(
        &mut self,
        client_id: String,
        access_token: String,
        refresh_token: String,
    ) {
        self.user.clear_user_id();
        self.auth
            .restore_session(client_id, access_token, refresh_token);
    }

    pub fn client_id(&self) -> Option<String> {
        self.auth.client_id()
    }

    pub fn access_token(&self) -> Option<String> {
        self.auth.access_token()
    }

    pub fn refresh_token(&self) -> Option<String> {
        self.auth.refresh_token()
    }

    // --- UserManager ---
    pub async fn fetch_current_user_id(&self) -> Result<String, SpotMpError> {
        self.execute_with_refresh(|| self.user.fetch_current_user_id())
            .await
    }

    pub fn clear_user_id(&self) {
        self.user.clear_user_id();
    }

    // --- PlaylistManager ---
    pub async fn create_playlist(
        &self,
        playlist_name: &str,
        public: bool,
        song_to_add: Option<&Song>,
    ) -> Result<Playlist, SpotMpError> {
        self.execute_with_refresh(|| {
            self.playlist
                .create_playlist(playlist_name, public, song_to_add)
        })
        .await
    }

    pub async fn add_song_to_playlist(
        &self,
        playlist_id: &str,
        song_id: &str,
    ) -> Result<(), SpotMpError> {
        self.execute_with_refresh(|| self.playlist.add_song_to_playlist(playlist_id, song_id))
            .await
    }

    pub async fn remove_song_from_playlist(
        &self,
        playlist_id: &str,
        song_id: &str,
    ) -> Result<(), SpotMpError> {
        self.execute_with_refresh(|| {
            self.playlist
                .remove_song_from_playlist(playlist_id, song_id)
        })
        .await
    }

    pub async fn unfollow_playlist(&self, playlist_id: &str) -> Result<(), SpotMpError> {
        self.execute_with_refresh(|| self.playlist.unfollow_playlist(playlist_id))
            .await
    }

    pub async fn get_playlist_tracks(&self, playlist_id: &str) -> Result<Vec<Song>, SpotMpError> {
        self.execute_with_refresh(|| self.playlist.get_playlist_tracks(playlist_id))
            .await
    }

    pub async fn import_liked_playlist(&self) -> Result<Vec<Playlist>, SpotMpError> {
        let user_id = self.fetch_current_user_id().await?;
        self.execute_with_refresh(|| self.playlist.import_liked_playlist(&user_id))
            .await
    }

    // --- AlbumManager ---
    pub async fn get_album_tracks(
        &self,
        album_id: &str,
        album_name: &str,
        release_year: Option<i32>,
        img_path: &str,
    ) -> Result<Vec<Song>, SpotMpError> {
        self.execute_with_refresh(|| {
            self.album
                .get_album_tracks(album_id, album_name, release_year, img_path)
        })
        .await
    }

    pub async fn import_liked_albums(&self) -> Result<Vec<Album>, SpotMpError> {
        self.execute_with_refresh(|| self.album.import_liked_albums())
            .await
    }

    pub async fn unfollow_album(&self, album_id: &str) -> Result<(), SpotMpError> {
        self.execute_with_refresh(|| self.album.unfollow_album(album_id))
            .await
    }

    // --- Combined Import (Playlists + Albums en paralelo) ---
    pub async fn import_liked_albums_and_playlist(
        &self,
    ) -> Result<(Vec<Playlist>, Vec<Album>), SpotMpError> {
        let user_id = self.fetch_current_user_id().await?;
        let playlists_fut = self.execute_with_refresh(|| self.playlist.import_liked_playlist(&user_id));
        let albums_fut = self.execute_with_refresh(|| self.album.import_liked_albums());
        try_join!(playlists_fut, albums_fut)
    }

    // --- SearchManager ---
    pub async fn search_query(
        &self,
        query: &str,
    ) -> Result<Vec<SpotifySearchResponse>, SpotMpError> {
        self.execute_with_refresh(|| self.search.search_query(query))
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicUsize, Ordering},
    };

    use super::*;

    use tokio::sync::{Barrier, Notify};

    #[derive(Default)]
    struct MockAuthState {
        client_id: Option<String>,
        access_token: Option<String>,
        refresh_token: Option<String>,
        session_epoch: u64,
        renew_calls: usize,
        renew_outcomes: VecDeque<Result<(), SpotMpError>>,
        yield_during_renew: bool,
        renew_started: Option<Arc<Notify>>,
        renew_release: Option<Arc<Notify>>,
    }

    #[derive(Clone, Default)]
    struct MockAuth {
        state: Arc<RwLock<MockAuthState>>,
    }

    impl MockAuth {
        fn push_renew_outcome(&self, outcome: Result<(), SpotMpError>) {
            self.state.write().unwrap().renew_outcomes.push_back(outcome);
        }

        fn set_yield_during_renew(&self, enabled: bool) {
            self.state.write().unwrap().yield_during_renew = enabled;
        }

        fn set_renew_gate(&self, started: Arc<Notify>, release: Arc<Notify>) {
            let mut s = self.state.write().unwrap();
            s.renew_started = Some(started);
            s.renew_release = Some(release);
        }

        fn renew_calls(&self) -> usize {
            self.state.read().unwrap().renew_calls
        }
    }

    impl AuthManager for MockAuth {
        async fn login(&mut self, client_id: &str) -> Result<(), SpotMpError> {
            let mut s = self.state.write().unwrap();
            s.client_id = Some(client_id.to_string());
            s.access_token = Some(format!("access-{client_id}"));
            s.refresh_token = Some(format!("refresh-{client_id}"));
            s.session_epoch = s.session_epoch.wrapping_add(1);
            Ok(())
        }

        fn restore_session(
            &mut self,
            client_id: String,
            access_token: String,
            refresh_token: String,
        ) {
            let mut s = self.state.write().unwrap();
            s.client_id = Some(client_id);
            s.access_token = Some(access_token);
            s.refresh_token = Some(refresh_token);
            s.session_epoch = s.session_epoch.wrapping_add(1);
        }

        fn logout(&self) {
            let mut s = self.state.write().unwrap();
            s.client_id = None;
            s.access_token = None;
            s.refresh_token = None;
            s.session_epoch = s.session_epoch.wrapping_add(1);
        }

        fn clear_tokens(&self) {
            let mut s = self.state.write().unwrap();
            s.access_token = None;
            s.refresh_token = None;
            s.session_epoch = s.session_epoch.wrapping_add(1);
        }

        async fn renew_token(&self) -> Result<(), SpotMpError> {
            let (epoch_before, should_yield, started, release) = {
                let s = self.state.read().unwrap();
                (
                    s.session_epoch,
                    s.yield_during_renew,
                    s.renew_started.clone(),
                    s.renew_release.clone(),
                )
            };
            if let Some(started) = started {
                started.notify_one();
            }
            if let Some(release) = release {
                release.notified().await;
            }
            if should_yield {
                tokio::task::yield_now().await;
            }
            let mut s = self.state.write().unwrap();
            s.renew_calls += 1;
            if let Some(outcome) = s.renew_outcomes.pop_front() {
                if outcome.is_ok() && s.session_epoch == epoch_before {
                    s.access_token = Some(format!("refreshed-{}", s.renew_calls));
                }
                return outcome;
            }
            if s.session_epoch != epoch_before {
                return Ok(());
            }
            if s.client_id.is_none() || s.refresh_token.is_none() {
                return Err(SpotMpError::SessionExpired(
                    "missing credentials".to_string(),
                ));
            }
            s.access_token = Some(format!("refreshed-{}", s.renew_calls));
            Ok(())
        }

        fn access_token(&self) -> Option<String> {
            self.state.read().unwrap().access_token.clone()
        }

        fn refresh_token(&self) -> Option<String> {
            self.state.read().unwrap().refresh_token.clone()
        }

        fn client_id(&self) -> Option<String> {
            self.state.read().unwrap().client_id.clone()
        }

        fn session_epoch(&self) -> u64 {
            self.state.read().unwrap().session_epoch
        }
    }

    struct MockUser {
        cached_id: RwLock<Option<String>>,
        next_fetched_id: Arc<RwLock<String>>,
        fetch_calls: AtomicUsize,
        clear_calls: AtomicUsize,
    }

    impl MockUser {
        fn new(initial_fetched_id: &str) -> (Self, Arc<RwLock<String>>) {
            let next_fetched_id = Arc::new(RwLock::new(initial_fetched_id.to_string()));
            let user = Self {
                cached_id: RwLock::new(None),
                next_fetched_id: Arc::clone(&next_fetched_id),
                fetch_calls: AtomicUsize::new(0),
                clear_calls: AtomicUsize::new(0),
            };
            (user, next_fetched_id)
        }
    }

    impl UserManager for MockUser {
        async fn fetch_current_user_id(&self) -> Result<String, SpotMpError> {
            if let Some(id) = self.cached_id.read().unwrap().clone() {
                return Ok(id);
            }
            self.fetch_calls.fetch_add(1, Ordering::SeqCst);
            let fetched = self.next_fetched_id.read().unwrap().clone();
            *self.cached_id.write().unwrap() = Some(fetched.clone());
            Ok(fetched)
        }

        fn clear_user_id(&self) {
            self.clear_calls.fetch_add(1, Ordering::SeqCst);
            *self.cached_id.write().unwrap() = None;
        }
    }

    struct TokenGate {
        auth: Option<MockAuth>,
        valid_token: Option<String>,
        first_call_barrier: Option<Arc<Barrier>>,
        calls: Arc<AtomicUsize>,
    }

    impl TokenGate {
        fn open() -> Self {
            Self {
                auth: None,
                valid_token: None,
                first_call_barrier: None,
                calls: Arc::new(AtomicUsize::new(0)),
            }
        }

        fn requiring(
            auth: MockAuth,
            valid_token: &str,
            first_call_barrier: Option<Arc<Barrier>>,
            calls: Arc<AtomicUsize>,
        ) -> Self {
            Self {
                auth: Some(auth),
                valid_token: Some(valid_token.to_string()),
                first_call_barrier,
                calls,
            }
        }

        async fn check(&self) -> Result<(), SpotMpError> {
            let call_num = self.calls.fetch_add(1, Ordering::SeqCst);
            if call_num == 0
                && let Some(barrier) = &self.first_call_barrier
            {
                barrier.wait().await;
            }
            if let (Some(auth), Some(expected)) = (&self.auth, &self.valid_token)
                && auth.access_token().as_deref() != Some(expected.as_str())
            {
                return Err(SpotMpError::SpotifyErrorStatus(Status::Unauthorized(
                    "token expired".to_string(),
                )));
            }
            Ok(())
        }
    }

    struct MockPlaylist {
        last_user_id: RwLock<Option<String>>,
        gate: TokenGate,
    }

    impl MockPlaylist {
        fn new() -> Self {
            Self {
                last_user_id: RwLock::new(None),
                gate: TokenGate::open(),
            }
        }

        fn requiring_token(
            auth: MockAuth,
            valid_token: &str,
            first_call_barrier: Option<Arc<Barrier>>,
        ) -> Self {
            Self {
                last_user_id: RwLock::new(None),
                gate: TokenGate::requiring(
                    auth,
                    valid_token,
                    first_call_barrier,
                    Arc::new(AtomicUsize::new(0)),
                ),
            }
        }
    }

    impl PlaylistManager for MockPlaylist {
        async fn create_playlist(
            &self,
            _playlist_name: &str,
            _public: bool,
            _song_to_add: Option<&Song>,
        ) -> Result<Playlist, SpotMpError> {
            unreachable!()
        }
        async fn add_song_to_playlist(
            &self,
            _playlist_id: &str,
            _song_id: &str,
        ) -> Result<(), SpotMpError> {
            Ok(())
        }
        async fn remove_song_from_playlist(
            &self,
            _playlist_id: &str,
            _song_id: &str,
        ) -> Result<(), SpotMpError> {
            Ok(())
        }
        async fn unfollow_playlist(&self, _playlist_id: &str) -> Result<(), SpotMpError> {
            Ok(())
        }
        async fn get_playlist_tracks(&self, _playlist_id: &str) -> Result<Vec<Song>, SpotMpError> {
            Ok(vec![])
        }
        async fn import_liked_playlist(&self, user_id: &str) -> Result<Vec<Playlist>, SpotMpError> {
            self.gate.check().await?;
            *self.last_user_id.write().unwrap() = Some(user_id.to_string());
            Ok(vec![])
        }
    }

    struct MockAlbum {
        gate: TokenGate,
    }

    impl MockAlbum {
        fn ok() -> Self {
            Self {
                gate: TokenGate::open(),
            }
        }

        fn requiring_token(
            auth: MockAuth,
            valid_token: &str,
            first_call_barrier: Option<Arc<Barrier>>,
        ) -> Self {
            Self {
                gate: TokenGate::requiring(
                    auth,
                    valid_token,
                    first_call_barrier,
                    Arc::new(AtomicUsize::new(0)),
                ),
            }
        }
    }

    impl AlbumManager for MockAlbum {
        async fn get_album_tracks(
            &self,
            _album_id: &str,
            _album_name: &str,
            _release_year: Option<i32>,
            _img_path: &str,
        ) -> Result<Vec<Song>, SpotMpError> {
            Ok(vec![])
        }
        async fn import_liked_albums(&self) -> Result<Vec<Album>, SpotMpError> {
            self.gate.check().await?;
            Ok(vec![])
        }
        async fn unfollow_album(&self, _album_id: &str) -> Result<(), SpotMpError> {
            Ok(())
        }
    }

    struct MockSearch {
        gate: TokenGate,
        scripted: Mutex<VecDeque<Result<Vec<SpotifySearchResponse>, SpotMpError>>>,
    }

    impl MockSearch {
        fn ok() -> Self {
            Self {
                gate: TokenGate::open(),
                scripted: Mutex::new(VecDeque::new()),
            }
        }

        fn requiring_token(auth: MockAuth, valid_token: &str) -> Self {
            Self::requiring_token_with_counter(auth, valid_token, Arc::new(AtomicUsize::new(0)))
        }

        fn requiring_token_with_counter(
            auth: MockAuth,
            valid_token: &str,
            calls: Arc<AtomicUsize>,
        ) -> Self {
            Self {
                gate: TokenGate::requiring(auth, valid_token, None, calls),
                scripted: Mutex::new(VecDeque::new()),
            }
        }

        fn with_responses(responses: Vec<Result<Vec<SpotifySearchResponse>, SpotMpError>>) -> Self {
            Self {
                gate: TokenGate::open(),
                scripted: Mutex::new(responses.into()),
            }
        }
    }

    impl SearchManager for MockSearch {
        async fn search_query(
            &self,
            _query: &str,
        ) -> Result<Vec<SpotifySearchResponse>, SpotMpError> {
            if let Some(response) = self.scripted.lock().unwrap().pop_front() {
                self.gate.calls.fetch_add(1, Ordering::SeqCst);
                return response;
            }
            self.gate.check().await?;
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn token_refresh_and_user_id_operations_work_through_shared_self() {
        let (mock_user, _) = MockUser::new("user-a");
        let mut client = SpotifyApiClient::with_services(
            MockAuth::default(),
            mock_user,
            MockPlaylist::new(),
            MockAlbum::ok(),
            MockSearch::ok(),
        );
        client.restore_session(
            "client-1".to_string(),
            "access-1".to_string(),
            "refresh-1".to_string(),
        );

        let shared: &SpotifyApiClient<MockAuth, MockUser, MockPlaylist, MockAlbum, MockSearch> =
            &client;

        // Fetch, clear, and use user ID through &self (including combined parallel import)
        assert_eq!(shared.fetch_current_user_id().await.unwrap(), "user-a");
        shared.clear_user_id();
        let _ = shared.import_liked_playlist().await.unwrap();
        let _ = shared.import_liked_albums_and_playlist().await.unwrap();
    }

    #[tokio::test]
    async fn login_and_restore_session_clear_stale_cached_user_id() {
        let (mock_user, next_user_id) = MockUser::new("user-alice");
        let mut client = SpotifyApiClient::with_services(
            MockAuth::default(),
            mock_user,
            MockPlaylist::new(),
            MockAlbum::ok(),
            MockSearch::ok(),
        );

        // First login caches Alice's user_id.
        client.login("client-alice").await.unwrap();
        assert_eq!(client.fetch_current_user_id().await.unwrap(), "user-alice");

        // Switching accounts via login must clear Alice's cached ID and fetch Bob's.
        *next_user_id.write().unwrap() = "user-bob".to_string();
        client.login("client-bob").await.unwrap();
        assert_eq!(client.fetch_current_user_id().await.unwrap(), "user-bob");

        // Switching accounts via restore_session must clear Bob's cached ID so Charlie is fetched.
        *next_user_id.write().unwrap() = "user-charlie".to_string();
        client.restore_session(
            "client-charlie".to_string(),
            "access-c".to_string(),
            "refresh-c".to_string(),
        );
        assert_eq!(
            client.fetch_current_user_id().await.unwrap(),
            "user-charlie"
        );
    }

    #[tokio::test]
    async fn logout_clears_client_id_tokens_and_cached_user_id() {
        let (mock_user, next_user_id) = MockUser::new("user-before-logout");
        let mut client = SpotifyApiClient::with_services(
            MockAuth::default(),
            mock_user,
            MockPlaylist::new(),
            MockAlbum::ok(),
            MockSearch::ok(),
        );

        client.login("client-1").await.unwrap();
        assert_eq!(client.client_id().as_deref(), Some("client-1"));
        assert!(client.access_token().is_some());
        assert!(client.refresh_token().is_some());
        assert_eq!(
            client.fetch_current_user_id().await.unwrap(),
            "user-before-logout"
        );

        client.logout();

        assert_eq!(client.client_id(), None);
        assert_eq!(client.access_token(), None);
        assert_eq!(client.refresh_token(), None);

        // Cached user_id was cleared, so next lookup fetches fresh value instead of stale cache.
        *next_user_id.write().unwrap() = "user-after-logout".to_string();
        assert_eq!(
            client.fetch_current_user_id().await.unwrap(),
            "user-after-logout"
        );
    }

    #[tokio::test]
    async fn reactive_401_refreshes_token_once_and_retries_operation_successfully() {
        let auth = MockAuth::default();
        let search = MockSearch::requiring_token(auth.clone(), "refreshed-1");
        let (mock_user, _) = MockUser::new("user-1");
        let mut client = SpotifyApiClient::with_services(
            auth.clone(),
            mock_user,
            MockPlaylist::new(),
            MockAlbum::ok(),
            search,
        );
        client.restore_session(
            "client-1".to_string(),
            "expired-access".to_string(),
            "refresh-1".to_string(),
        );

        let result = client.search_query("daft punk").await;

        assert!(result.is_ok(), "expected retry after 401 refresh to succeed, got: {result:?}");
        assert_eq!(auth.renew_calls(), 1);
        assert_eq!(client.access_token().as_deref(), Some("refreshed-1"));
    }

    #[tokio::test]
    async fn happy_path_completes_with_zero_refreshes() {
        let auth = MockAuth::default();
        let (mock_user, _) = MockUser::new("user-1");
        let mut client = SpotifyApiClient::with_services(
            auth.clone(),
            mock_user,
            MockPlaylist::new(),
            MockAlbum::ok(),
            MockSearch::ok(),
        );
        client.restore_session(
            "client-1".to_string(),
            "valid-access".to_string(),
            "refresh-1".to_string(),
        );

        let result = client.search_query("radiohead").await;

        assert!(result.is_ok());
        assert_eq!(auth.renew_calls(), 0);
        assert_eq!(client.access_token().as_deref(), Some("valid-access"));
    }

    #[tokio::test]
    async fn non_401_errors_bypass_refresh_and_propagate_directly() {
        let reqwest_err = reqwest::Client::new().get("://bad-url").build().unwrap_err();
        let non_401_errors = vec![
            SpotMpError::SpotifyErrorStatus(Status::BadRequest("bad query".to_string())),
            SpotMpError::SpotifyErrorStatus(Status::Unknown(403, "forbidden".to_string())),
            SpotMpError::SpotifyErrorStatus(Status::TooManyRequest("rate limited".to_string())),
            SpotMpError::SpotifyErrorStatus(Status::Unknown(503, "unavailable".to_string())),
            SpotMpError::Reqwest(reqwest_err),
        ];

        let auth = MockAuth::default();
        let (mock_user, _) = MockUser::new("user-1");
        let search = MockSearch::with_responses(non_401_errors.into_iter().map(Err).collect());
        let mut client = SpotifyApiClient::with_services(
            auth.clone(),
            mock_user,
            MockPlaylist::new(),
            MockAlbum::ok(),
            search,
        );
        client.restore_session(
            "client-1".to_string(),
            "valid-access".to_string(),
            "refresh-1".to_string(),
        );

        assert!(matches!(
            client.search_query("q").await,
            Err(SpotMpError::SpotifyErrorStatus(Status::BadRequest(_)))
        ));
        assert!(matches!(
            client.search_query("q").await,
            Err(SpotMpError::SpotifyErrorStatus(Status::Unknown(403, _)))
        ));
        assert!(matches!(
            client.search_query("q").await,
            Err(SpotMpError::SpotifyErrorStatus(Status::TooManyRequest(_)))
        ));
        assert!(matches!(
            client.search_query("q").await,
            Err(SpotMpError::SpotifyErrorStatus(Status::Unknown(503, _)))
        ));
        assert!(matches!(
            client.search_query("q").await,
            Err(SpotMpError::Reqwest(_))
        ));

        assert_eq!(auth.renew_calls(), 0);
        assert_eq!(client.access_token().as_deref(), Some("valid-access"));
        assert_eq!(client.refresh_token().as_deref(), Some("refresh-1"));
    }

    #[tokio::test]
    async fn double_401_caps_at_one_retry_and_preserves_tokens() {
        let auth = MockAuth::default();
        let (mock_user, _) = MockUser::new("user-1");
        let search = MockSearch::with_responses(vec![
            Err(SpotMpError::SpotifyErrorStatus(Status::Unauthorized(
                "expired".to_string(),
            ))),
            Err(SpotMpError::SpotifyErrorStatus(Status::Unauthorized(
                "insufficient scope".to_string(),
            ))),
        ]);
        let mut client = SpotifyApiClient::with_services(
            auth.clone(),
            mock_user,
            MockPlaylist::new(),
            MockAlbum::ok(),
            search,
        );
        client.restore_session(
            "client-1".to_string(),
            "access-1".to_string(),
            "refresh-1".to_string(),
        );

        let err = client.search_query("q").await.unwrap_err();

        assert!(matches!(
            err,
            SpotMpError::SpotifyErrorStatus(Status::Unauthorized(ref msg)) if msg == "insufficient scope"
        ));
        assert_eq!(auth.renew_calls(), 1);
        assert_eq!(client.access_token().as_deref(), Some("refreshed-1"));
        assert_eq!(client.refresh_token().as_deref(), Some("refresh-1"));
        assert_eq!(client.client_id().as_deref(), Some("client-1"));
    }

    #[tokio::test]
    async fn terminal_refresh_failure_clears_tokens_and_user_id_while_preserving_client_id() {
        for terminal_error in [
            SpotMpError::SpotifyErrorStatus(Status::BadRequest("invalid_grant".to_string())),
            SpotMpError::SpotifyErrorStatus(Status::Unauthorized("invalid_client".to_string())),
            SpotMpError::SessionExpired("missing refresh_token".to_string()),
        ] {
            let auth = MockAuth::default();
            auth.push_renew_outcome(Err(terminal_error));
            let (mock_user, next_user_id) = MockUser::new("user-before-expiry");
            let search = MockSearch::with_responses(vec![Err(SpotMpError::SpotifyErrorStatus(
                Status::Unauthorized("expired".to_string()),
            ))]);
            let mut client = SpotifyApiClient::with_services(
                auth.clone(),
                mock_user,
                MockPlaylist::new(),
                MockAlbum::ok(),
                search,
            );
            client.restore_session(
                "client-keep-me".to_string(),
                "expired-access".to_string(),
                "revoked-refresh".to_string(),
            );
            // Warm cached user_id before expiry.
            assert_eq!(
                client.fetch_current_user_id().await.unwrap(),
                "user-before-expiry"
            );

            let err = client.search_query("q").await.unwrap_err();

            assert!(
                matches!(err, SpotMpError::SessionExpired(_)),
                "expected SessionExpired for terminal refresh failure, got: {err:?}"
            );
            assert_eq!(client.access_token(), None);
            assert_eq!(client.refresh_token(), None);
            assert_eq!(client.client_id().as_deref(), Some("client-keep-me"));

            // Verify cached user_id was cleared (next fetch gets fresh identity instead of cached).
            *next_user_id.write().unwrap() = "user-after-relogin".to_string();
            assert_eq!(
                client.fetch_current_user_id().await.unwrap(),
                "user-after-relogin"
            );
        }
    }

    #[tokio::test]
    async fn transient_refresh_failure_propagates_error_and_preserves_tokens() {
        let reqwest_err = reqwest::Client::new().get("://bad-url").build().unwrap_err();
        let serde_err = serde_json::from_str::<serde_json::Value>("not-json").unwrap_err();

        let transient_cases: Vec<(SpotMpError, fn(&SpotMpError) -> bool)> = vec![
            (
                SpotMpError::Reqwest(reqwest_err),
                |e| matches!(e, SpotMpError::Reqwest(_)),
            ),
            (
                SpotMpError::SpotifyErrorStatus(Status::TooManyRequest("retry later".to_string())),
                |e| matches!(e, SpotMpError::SpotifyErrorStatus(Status::TooManyRequest(_))),
            ),
            (
                SpotMpError::SpotifyErrorStatus(Status::Unknown(502, "bad gateway".to_string())),
                |e| matches!(e, SpotMpError::SpotifyErrorStatus(Status::Unknown(502, _))),
            ),
            (
                SpotMpError::SerdeJsonError(serde_err),
                |e| matches!(e, SpotMpError::SerdeJsonError(_)),
            ),
        ];

        for (transient_error, matches_expected) in transient_cases {
            let auth = MockAuth::default();
            auth.push_renew_outcome(Err(transient_error));
            let (mock_user, next_user_id) = MockUser::new("user-cached");
            let search = MockSearch::with_responses(vec![Err(SpotMpError::SpotifyErrorStatus(
                Status::Unauthorized("expired".to_string()),
            ))]);
            let mut client = SpotifyApiClient::with_services(
                auth.clone(),
                mock_user,
                MockPlaylist::new(),
                MockAlbum::ok(),
                search,
            );
            client.restore_session(
                "client-1".to_string(),
                "access-1".to_string(),
                "refresh-1".to_string(),
            );
            assert_eq!(client.fetch_current_user_id().await.unwrap(), "user-cached");

            let err = client.search_query("q").await.unwrap_err();

            assert!(
                matches_expected(&err),
                "expected original transient error to propagate, got: {err:?}"
            );
            assert_eq!(client.access_token().as_deref(), Some("access-1"));
            assert_eq!(client.refresh_token().as_deref(), Some("refresh-1"));
            assert_eq!(client.client_id().as_deref(), Some("client-1"));

            // Cached user_id remains intact (no forced logout/clear on transient failure).
            *next_user_id.write().unwrap() = "user-changed".to_string();
            assert_eq!(client.fetch_current_user_id().await.unwrap(), "user-cached");
        }
    }

    #[tokio::test]
    async fn concurrent_401s_deduplicate_refresh_behind_single_flight_lock() {
        let auth = MockAuth::default();
        auth.set_yield_during_renew(true);
        let barrier = Arc::new(Barrier::new(2));
        let (mock_user, _) = MockUser::new("user-1");
        let playlist = MockPlaylist::requiring_token(
            auth.clone(),
            "refreshed-1",
            Some(Arc::clone(&barrier)),
        );
        let album = MockAlbum::requiring_token(
            auth.clone(),
            "refreshed-1",
            Some(Arc::clone(&barrier)),
        );
        let mut client = SpotifyApiClient::with_services(
            auth.clone(),
            mock_user,
            playlist,
            album,
            MockSearch::ok(),
        );
        client.restore_session(
            "client-1".to_string(),
            "expired-access".to_string(),
            "refresh-1".to_string(),
        );

        let result = client.import_liked_albums_and_playlist().await;

        assert!(result.is_ok(), "expected parallel try_join! import to succeed, got: {result:?}");
        assert_eq!(
            auth.renew_calls(),
            1,
            "concurrent 401s must deduplicate to a single renew_token() call"
        );
        assert_eq!(client.access_token().as_deref(), Some("refreshed-1"));
    }

    #[tokio::test]
    async fn logout_during_in_flight_refresh_prevents_stale_token_overwrite_and_retry() {
        let auth = MockAuth::default();
        let renew_started = Arc::new(Notify::new());
        let renew_release = Arc::new(Notify::new());
        auth.set_renew_gate(Arc::clone(&renew_started), Arc::clone(&renew_release));

        let search_calls = Arc::new(AtomicUsize::new(0));
        let search = MockSearch::requiring_token_with_counter(
            auth.clone(),
            "refreshed-1",
            Arc::clone(&search_calls),
        );
        let (mock_user, _) = MockUser::new("user-1");
        let mut client = SpotifyApiClient::with_services(
            auth.clone(),
            mock_user,
            MockPlaylist::new(),
            MockAlbum::ok(),
            search,
        );
        client.restore_session(
            "client-1".to_string(),
            "expired-access".to_string(),
            "refresh-1".to_string(),
        );
        let client = Arc::new(client);

        let client_for_req = Arc::clone(&client);
        let req_handle = tokio::spawn(async move { client_for_req.search_query("q").await });

        // Wait until renew_token() is in flight, then log out concurrently.
        renew_started.notified().await;
        client.logout();
        renew_release.notify_one();

        let err = req_handle.await.unwrap().unwrap_err();
        assert!(
            matches!(err, SpotMpError::SessionExpired(_)),
            "expected SessionExpired when logout races in-flight refresh, got: {err:?}"
        );
        assert_eq!(client.access_token(), None);
        assert_eq!(client.refresh_token(), None);
        assert_eq!(client.client_id(), None);
        assert_eq!(
            search_calls.load(Ordering::SeqCst),
            1,
            "stale request must not retry after logout invalidated the session"
        );
    }

    #[tokio::test]
    async fn stale_terminal_refresh_failure_after_session_change_does_not_wipe_new_session() {
        let auth = MockAuth::default();
        auth.push_renew_outcome(Err(SpotMpError::SpotifyErrorStatus(Status::BadRequest(
            "invalid_grant".to_string(),
        ))));
        let renew_started = Arc::new(Notify::new());
        let renew_release = Arc::new(Notify::new());
        auth.set_renew_gate(Arc::clone(&renew_started), Arc::clone(&renew_release));

        let search = MockSearch::with_responses(vec![Err(SpotMpError::SpotifyErrorStatus(
            Status::Unauthorized("expired".to_string()),
        ))]);
        let (mock_user, _) = MockUser::new("user-1");
        let mut client = SpotifyApiClient::with_services(
            auth.clone(),
            mock_user,
            MockPlaylist::new(),
            MockAlbum::ok(),
            search,
        );
        client.restore_session(
            "client-old".to_string(),
            "old-access".to_string(),
            "old-refresh".to_string(),
        );
        let client = Arc::new(client);

        let client_for_req = Arc::clone(&client);
        let req_handle = tokio::spawn(async move { client_for_req.search_query("q").await });

        // Wait until the old session's renew_token() is in flight, then log out and establish a new session.
        renew_started.notified().await;
        client.logout();
        {
            let mut s = auth.state.write().unwrap();
            s.client_id = Some("client-new".to_string());
            s.access_token = Some("new-access".to_string());
            s.refresh_token = Some("new-refresh".to_string());
            s.session_epoch = s.session_epoch.wrapping_add(1);
        }
        renew_release.notify_one();

        let err = req_handle.await.unwrap().unwrap_err();
        assert!(matches!(err, SpotMpError::SessionExpired(_)));
        assert_eq!(client.client_id().as_deref(), Some("client-new"));
        assert_eq!(client.access_token().as_deref(), Some("new-access"));
        assert_eq!(client.refresh_token().as_deref(), Some("new-refresh"));
    }
}