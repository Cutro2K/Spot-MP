use std::sync::{Arc, RwLock};
use futures::try_join;

use crate::{
    domain::{
        album::Album,
        playlist::{Playlist, SpotifySearchResponse},
        song::Song,
        spot_mp_errors::SpotMpError,
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
        }
    }

    // --- AuthManager ---
    pub async fn login(&mut self, client_id: &str) -> Result<(), SpotMpError> {
        self.user.clear_user_id();
        self.auth.login(client_id).await?;
        let _ = self.user.fetch_current_user_id().await;
        Ok(())
    }

    pub fn logout(&mut self) {
        self.auth.logout();
        self.user.clear_user_id();
    }

    pub async fn renew_token(&self) -> Result<(), SpotMpError> {
        self.auth.renew_token().await
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
        self.user.fetch_current_user_id().await
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
        self.playlist
            .create_playlist(playlist_name, public, song_to_add)
            .await
    }

    pub async fn add_song_to_playlist(
        &self,
        playlist_id: &str,
        song_id: &str,
    ) -> Result<(), SpotMpError> {
        self.playlist.add_song_to_playlist(playlist_id, song_id).await
    }

    pub async fn remove_song_from_playlist(
        &self,
        playlist_id: &str,
        song_id: &str,
    ) -> Result<(), SpotMpError> {
        self.playlist
            .remove_song_from_playlist(playlist_id, song_id)
            .await
    }

    pub async fn unfollow_playlist(&self, playlist_id: &str) -> Result<(), SpotMpError> {
        self.playlist.unfollow_playlist(playlist_id).await
    }

    pub async fn get_playlist_tracks(&self, playlist_id: &str) -> Result<Vec<Song>, SpotMpError> {
        self.playlist.get_playlist_tracks(playlist_id).await
    }

    pub async fn import_liked_playlist(&self) -> Result<Vec<Playlist>, SpotMpError> {
        let user_id = self.user.fetch_current_user_id().await?;
        self.playlist.import_liked_playlist(&user_id).await
    }

    // --- AlbumManager ---
    pub async fn get_album_tracks(
        &self,
        album_id: &str,
        album_name: &str,
        release_year: Option<i32>,
        img_path: &str,
    ) -> Result<Vec<Song>, SpotMpError> {
        self.album
            .get_album_tracks(album_id, album_name, release_year, img_path)
            .await
    }

    pub async fn import_liked_albums(&self) -> Result<Vec<Album>, SpotMpError> {
        self.album.import_liked_albums().await
    }

    pub async fn unfollow_album(&self, album_id: &str) -> Result<(), SpotMpError> {
        self.album.unfollow_album(album_id).await
    }

    // --- Combined Import (Playlists + Albums en paralelo) ---
    pub async fn import_liked_albums_and_playlist(
        &self,
    ) -> Result<(Vec<Playlist>, Vec<Album>), SpotMpError> {
        let user_id = self.user.fetch_current_user_id().await?;
        let playlists_fut = self.playlist.import_liked_playlist(&user_id);
        let albums_fut = self.album.import_liked_albums();
        try_join!(playlists_fut, albums_fut)
    }

    // --- SearchManager ---
    pub async fn search_query(
        &self,
        query: &str,
    ) -> Result<Vec<SpotifySearchResponse>, SpotMpError> {
        self.search.search_query(query).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc, RwLock,
        atomic::{AtomicUsize, Ordering},
    };

    use super::*;

    #[derive(Default)]
    struct MockAuthState {
        client_id: Option<String>,
        access_token: Option<String>,
        refresh_token: Option<String>,
        renew_calls: usize,
    }

    #[derive(Clone, Default)]
    struct MockAuth {
        state: Arc<RwLock<MockAuthState>>,
    }

    impl AuthManager for MockAuth {
        async fn login(&mut self, client_id: &str) -> Result<(), SpotMpError> {
            let mut s = self.state.write().unwrap();
            s.client_id = Some(client_id.to_string());
            s.access_token = Some(format!("access-{client_id}"));
            s.refresh_token = Some(format!("refresh-{client_id}"));
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
        }

        fn logout(&mut self) {
            let mut s = self.state.write().unwrap();
            s.client_id = None;
            s.access_token = None;
            s.refresh_token = None;
        }

        fn clear_tokens(&self) {
            let mut s = self.state.write().unwrap();
            s.access_token = None;
            s.refresh_token = None;
        }

        async fn renew_token(&self) -> Result<(), SpotMpError> {
            let mut s = self.state.write().unwrap();
            if s.client_id.is_none() || s.refresh_token.is_none() {
                return Err(SpotMpError::SessionExpired(
                    "missing credentials".to_string(),
                ));
            }
            s.renew_calls += 1;
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

    struct MockPlaylist {
        last_user_id: RwLock<Option<String>>,
    }

    impl MockPlaylist {
        fn new() -> Self {
            Self {
                last_user_id: RwLock::new(None),
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
            *self.last_user_id.write().unwrap() = Some(user_id.to_string());
            Ok(vec![])
        }
    }

    struct MockAlbum;
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
            Ok(vec![])
        }
        async fn unfollow_album(&self, _album_id: &str) -> Result<(), SpotMpError> {
            Ok(())
        }
    }

    struct MockSearch;
    impl SearchManager for MockSearch {
        async fn search_query(
            &self,
            _query: &str,
        ) -> Result<Vec<SpotifySearchResponse>, SpotMpError> {
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
            MockAlbum,
            MockSearch,
        );
        client.restore_session(
            "client-1".to_string(),
            "access-1".to_string(),
            "refresh-1".to_string(),
        );

        let shared: &SpotifyApiClient<MockAuth, MockUser, MockPlaylist, MockAlbum, MockSearch> =
            &client;

        // Renew token through &self without passing client_id
        shared.renew_token().await.unwrap();
        assert_eq!(shared.access_token().as_deref(), Some("refreshed-1"));

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
            MockAlbum,
            MockSearch,
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
            MockAlbum,
            MockSearch,
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
}