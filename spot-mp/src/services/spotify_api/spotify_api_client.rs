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
        self.auth.login(client_id).await?;
        let _ = self.user.fetch_current_user_id().await;
        Ok(())
    }

    pub fn logout(&mut self) {
        self.auth.logout();
        self.user.clear_user_id();
    }

    pub async fn renew_token(&mut self, client_id: &str) -> Result<(), SpotMpError> {
        self.auth.renew_token(client_id).await
    }

    pub fn set_tokens(&mut self, access_token: Option<String>, refresh_token: Option<String>) {
        self.auth.set_tokens(access_token, refresh_token);
    }

    pub fn access_token(&self) -> Option<String> {
        self.auth.access_token()
    }

    pub fn refresh_token(&self) -> Option<String> {
        self.auth.refresh_token()
    }

    // --- UserManager ---
    pub async fn fetch_current_user_id(&mut self) -> Result<String, SpotMpError> {
        self.user.fetch_current_user_id().await
    }

    pub fn clear_user_id(&mut self) {
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

    pub async fn import_liked_playlist(&mut self) -> Result<Vec<Playlist>, SpotMpError> {
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
        &mut self,
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