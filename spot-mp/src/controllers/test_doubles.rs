use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use tokio::sync::mpsc;

use crate::controllers::auth_controller::AuthController;
use crate::controllers::controller_hub::ControllerHub;
use crate::controllers::local_controller::LocalController;
use crate::controllers::music_player_controller::MusicPlayerController;
use crate::controllers::spotify_controller::SpotifyController;
use crate::domain::{
    album::Album,
    audio_messages::{Event, Messages},
    auth_models::{LibrespotData, SessionData, WebApiData},
    playlist::{Playlist, SpotifySearchResponse},
    song::Song,
    spot_mp_errors::SpotMpError,
};
use crate::services::data_storage::auth_storage::AuthStorage;
use crate::services::spotify_api::traits::{
    AlbumManager, AuthManager, PlaylistManager, SearchManager, SpotifyClient, UserManager,
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

pub(crate) struct TempDirGuard {
    path: PathBuf,
}

impl TempDirGuard {
    pub(crate) fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "spot-mp-hub-test-{}-{}",
            name,
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&path).expect("create temp test dir");
        Self { path }
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

pub(crate) struct TestHubHarness<C: SpotifyClient> {
    pub(crate) hub: ControllerHub<C>,
    pub(crate) dispatched_rx: mpsc::UnboundedReceiver<crate::state_store::app::action::Action>,
    pub(crate) audio_msg_rx: mpsc::UnboundedReceiver<Messages>,
    _audio_evt_tx: mpsc::UnboundedSender<Event>,
    _hub_in_tx: mpsc::UnboundedSender<crate::state_store::app::action::Action>,
    temp_dir: TempDirGuard,
}

impl<C: SpotifyClient> TestHubHarness<C> {
    pub(crate) fn with_client(name: &str, initial_session: SessionData, client: C) -> Self {
        let temp_dir = TempDirGuard::new(name);
        let storage = AuthStorage::new(temp_dir.path.clone());
        storage
            .save_session(initial_session.clone())
            .expect("seed initial session.json");

        let local_controller =
            LocalController::with_config_folder(temp_dir.path.clone()).expect("local ctrl");
        let auth_controller = AuthController::new(temp_dir.path.clone());

        let mut spotify_controller = SpotifyController::with_client(client);
        if let (Some(client_id), Some(web_api)) =
            (initial_session.client_id, initial_session.web_api)
        {
            spotify_controller.restore_session(client_id, web_api);
        }

        let (audio_msg_tx, audio_msg_rx) = mpsc::unbounded_channel();
        let (audio_evt_tx, audio_evt_rx) = mpsc::unbounded_channel();
        let music_player_controller =
            MusicPlayerController::with_channels(audio_msg_tx, audio_evt_rx);

        let (hub_in_tx, hub_in_rx) = mpsc::unbounded_channel();
        let (dispatched_tx, dispatched_rx) = mpsc::unbounded_channel();

        let hub = ControllerHub::with_controllers(
            hub_in_rx,
            dispatched_tx,
            local_controller,
            spotify_controller,
            music_player_controller,
            auth_controller,
        );

        Self {
            hub,
            dispatched_rx,
            audio_msg_rx,
            _audio_evt_tx: audio_evt_tx,
            _hub_in_tx: hub_in_tx,
            temp_dir,
        }
    }

    pub(crate) fn read_session_from_disk(&self) -> SessionData {
        AuthStorage::new(self.temp_dir.path.clone())
            .load_session()
            .expect("load session.json from disk")
    }
}

pub(crate) fn seeded_session() -> SessionData {
    SessionData {
        client_id: Some("client-123".to_string()),
        web_api: Some(WebApiData {
            access_token: "access-v1".to_string(),
            refresh_token: "refresh-v1".to_string(),
        }),
        librespot: Some(LibrespotData {
            device_id: "device-xyz".to_string(),
            credentials: "creds-blob".to_string(),
        }),
    }
}
