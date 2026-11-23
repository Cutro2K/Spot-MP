use crate::domain::{
    album::Album, playlist::{ Playlist, SpotifySearchResponse }, song::Song, spot_mp_errors::SpotMpError
};

// --- Spotify Controller traits ---
pub trait SearchManager {
    fn search_query(
        &self, 
        query: &str
    ) -> impl Future<Output = Result<Vec<SpotifySearchResponse>, SpotMpError>> + Send + Sync;
}

pub trait PlaylistManager {
    fn create_playlist(
        &self, 
        playlist_name: &str, 
        public: bool, 
        song_to_add: Option<&Song>
    ) -> impl Future<Output = Result<Playlist, SpotMpError>> + Send + Sync;  

    fn add_song_to_playlist(
        &self, 
        playlist_id: &str, 
        song_id: &str
    ) -> impl Future<Output = Result<(), SpotMpError>> + Send + Sync;

    fn remove_song_from_playlist(
        &self,
        playlist_id: &str,
        song_id: &str,
    ) -> impl Future<Output = Result<(), SpotMpError>> + Send + Sync;

    fn unfollow_playlist(
        &self, 
        playlist_id: &str
    ) -> impl Future<Output = Result<(), SpotMpError>> + Send + Sync;

    fn get_playlist_tracks(
        &self, 
        playlist_id: &str
    ) -> impl Future<Output = Result<Vec<Song>, SpotMpError>> + Send + Sync;

    fn import_liked_playlist(
        &self,
        user_id: &str
    ) -> impl Future<Output = Result<Vec<Playlist>, SpotMpError>> + Send + Sync;
}

pub trait AlbumManager {
    fn get_album_tracks(
        &self,
        album_id: &str,
        album_name: &str,
        release_year: Option<i32>,
        img_path: &str,
    ) -> impl Future<Output = Result<Vec<Song>, SpotMpError>> + Send + Sync;

    fn import_liked_albums(
        &self
    ) -> impl Future<Output = Result<Vec<Album>, SpotMpError>> + Send + Sync;

    fn unfollow_album(
        &self, 
        album_id: &str
    ) -> impl Future<Output = Result<(), SpotMpError>> + Send + Sync;
}

pub trait AuthManager {
    fn login(
        &mut self,
        client_id: &str
    ) -> impl Future<Output = Result<(), SpotMpError>> + Send + Sync;

    fn restore_session(
        &mut self,
        client_id: String,
        access_token: String,
        refresh_token: String,
    );

    fn logout(
        &self
    );

    fn clear_tokens(
        &self
    );

    fn renew_token(
        &self
    ) -> impl Future<Output = Result<(), SpotMpError>> + Send + Sync; 

    fn access_token(&self) -> Option<String>;
    fn refresh_token(&self) -> Option<String>;
    fn client_id(&self) -> Option<String>;
    fn session_epoch(&self) -> u64;
}

pub trait UserManager {
    fn fetch_current_user_id(
        &self
    ) -> impl Future<Output = Result<String, SpotMpError>> + Send + Sync;

    fn clear_user_id(
        &self
    );
}
pub trait BrowserOpener: Send + Sync {
    fn open(&self, url: &str) -> Result<(), SpotMpError>;
}