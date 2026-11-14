use crate::domain::{
    playlist::{ Playlist, SpotifySearchResponse }, 
    song::Song, 
    spot_mp_errors::SpotMpError
};

// --- Spotify Controller traits ---
pub trait SpotifySearcher {
    fn search_query(
        &self, 
        query: &str
    ) -> impl Future<Output = Result<Vec<SpotifySearchResponse>, SpotMpError>> + Send;
}

pub trait SpotifyPlaylistManager {
    fn create_playlist(
        &self, 
        playlist_name: &str, 
        public: bool, 
        song_to_add: Option<&Song>
    ) -> impl Future<Output = Result<Playlist, SpotMpError>> + Send;  

    fn add_song_to_playlist(
        &self, 
        playlist_id: &str, 
        song_id: &str
    ) -> impl Future<Output = Result<(), SpotMpError>> + Send;

    fn remove_song_from_playlist(
        &self,
        playlist_id: &str,
        song_id: &str,
    ) -> impl Future<Output = Result<(), SpotMpError>> + Send;
    
    fn unfollow_playlist(
        &self, 
        playlist_id: &str
    ) -> impl Future<Output = Result<(), SpotMpError>> + Send;
}

pub trait SpotifyAuthenticator {
    fn login(
        &mut self,
        client_id: &str
    ) -> impl Future<Output = Result<(), SpotMpError>> + Send;

    fn logout(
        &mut self
    );

    fn renew_token(
        &mut self, 
        client_id: &str
    ) -> impl Future<Output = Result<(), SpotMpError>> + Send; 

    fn set_tokens(
        &mut self, 
        access_token: Option<String>, 
        refresh_token: Option<String>
    );
    fn access_token(&self) -> Option<String>;
    fn refresh_token(&self) -> Option<String>;
}

pub trait SpotifyLibrary {
    fn import_liked_albums_and_playlist(
        &mut self
    ) -> impl Future<Output = Result<Vec<Playlist>, SpotMpError>> + Send;
}

// En traits.rs
pub trait BrowserOpener: Send + Sync {
    fn open(&self, url: &str) -> Result<(), SpotMpError>;
}