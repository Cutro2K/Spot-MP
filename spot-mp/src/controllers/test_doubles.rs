use std::collections::VecDeque;
use std::sync::{Arc, Mutex, RwLock};

use crate::domain::{
    album::Album,
    playlist::{Playlist, SpotifySearchResponse},
    song::Song,
    spot_mp_errors::SpotMpError,
};
use crate::services::spotify_api::traits::{
    AlbumManager, AuthManager, PlaylistManager, SearchManager, UserManager,
};

#[derive(Default)]
struct StubAuthState {
    client_id: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<String>,
    session_epoch: u64,
    renew_outcomes: VecDeque<Result<(String, String), SpotMpError>>,
}

#[derive(Clone, Default)]
pub(crate) struct StubAuth {
    state: Arc<RwLock<StubAuthState>>,
}

impl StubAuth {
    pub(crate) fn push_renew_outcome(&self, outcome: Result<(String, String), SpotMpError>) {
        self.state.write().unwrap().renew_outcomes.push_back(outcome);
    }
}

impl AuthManager for StubAuth {
    async fn login(&mut self, client_id: &str) -> Result<(), SpotMpError> {
        let mut s = self.state.write().unwrap();
        s.client_id = Some(client_id.to_string());
        s.access_token = Some("access-login".to_string());
        s.refresh_token = Some("refresh-login".to_string());
        s.session_epoch += 1;
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
        s.session_epoch += 1;
    }

    fn logout(&self) {
        let mut s = self.state.write().unwrap();
        s.client_id = None;
        s.access_token = None;
        s.refresh_token = None;
        s.session_epoch += 1;
    }

    fn clear_tokens(&self) {
        let mut s = self.state.write().unwrap();
        s.access_token = None;
        s.refresh_token = None;
        s.session_epoch += 1;
    }

    async fn renew_token(&self) -> Result<(), SpotMpError> {
        let outcome = self.state.write().unwrap().renew_outcomes.pop_front();
        match outcome {
            Some(Ok((access, refresh))) => {
                let mut s = self.state.write().unwrap();
                s.access_token = Some(access);
                s.refresh_token = Some(refresh);
                Ok(())
            }
            Some(Err(err)) => Err(err),
            None => Err(SpotMpError::SessionExpired(
                "no scripted refresh".to_string(),
            )),
        }
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

pub(crate) struct StubUser;

impl UserManager for StubUser {
    async fn fetch_current_user_id(&self) -> Result<String, SpotMpError> {
        Ok("user-1".to_string())
    }

    fn clear_user_id(&self) {}
}

pub(crate) struct StubPlaylist {
    responses: Mutex<VecDeque<Result<Vec<Playlist>, SpotMpError>>>,
}

impl StubPlaylist {
    pub(crate) fn with_responses(responses: Vec<Result<Vec<Playlist>, SpotMpError>>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
        }
    }
}

impl PlaylistManager for StubPlaylist {
    async fn create_playlist(
        &self,
        _playlist_name: &str,
        _public: bool,
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

    async fn import_liked_playlist(&self, _user_id: &str) -> Result<Vec<Playlist>, SpotMpError> {
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Ok(vec![]))
    }
}

pub(crate) struct StubAlbum;

impl AlbumManager for StubAlbum {
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

pub(crate) struct StubSearch {
    responses: Mutex<VecDeque<Result<Vec<SpotifySearchResponse>, SpotMpError>>>,
}

impl StubSearch {
    pub(crate) fn with_responses(
        responses: Vec<Result<Vec<SpotifySearchResponse>, SpotMpError>>,
    ) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
        }
    }
}

impl SearchManager for StubSearch {
    async fn search_query(
        &self,
        _query: &str,
    ) -> Result<Vec<SpotifySearchResponse>, SpotMpError> {
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Ok(vec![]))
    }
}
