use log::{error, info};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::controllers::auth_controller::AuthController;
use crate::controllers::local_controller::LocalController;
use crate::controllers::music_player_controller::MusicPlayerController;
use crate::controllers::spotify_controller::SpotifyController;
use crate::services::spotify_api::spotify_api_client::SpotifyApiClient;
use crate::services::spotify_api::traits::SpotifyClient;
use crate::state_store::app::action::{Action, LocalAction, MusicPlayerAction, SpotifyAction};

// ControllerHub communicates with dispatcher to handle backend calls and produce actions with changes
// to the frontend; some functions will execute synchronously and others asynchronously depending on
// whether the function has to mutate the internal values of the attributes.

#[allow(dead_code)]
pub struct ControllerHub<C: SpotifyClient = SpotifyApiClient> {
    reciever: UnboundedReceiver<Action>, // Comes from the dispatcher
    sender: UnboundedSender<Action>,     // Goes to the dispatcher
    local_controller: LocalController,
    spotify_controller: Arc<RwLock<SpotifyController<C>>>,
    music_player_controller: MusicPlayerController,
    auth_controller: AuthController,
}

impl ControllerHub {
    pub fn new(
        dispatcher_reciever: UnboundedReceiver<Action>,
        dispatcher_sender: UnboundedSender<Action>,
    ) -> Self {
        let local_c = LocalController::new().unwrap_or_else(|e| {
            error!("Instancing ControllerHub class(local controller): {}", e);
            panic!("Failed Instancing ControllerHub class!");
        });
        let mut spotify_c = SpotifyController::new();
        let music_player_c = MusicPlayerController::new();
        let mut auth_c = AuthController::new(local_c.get_spot_mp_folder());

        // Restore previously saved session if available
        let saved_session = auth_c.load_session();
        if let Some(client_id) = saved_session.client_id.clone() {
            if let Some(web_api) = saved_session.web_api.clone() {
                spotify_c.restore_session(client_id.clone(), web_api);
            }
            if saved_session.librespot.is_some() {
                music_player_c.login(&client_id, saved_session.librespot);
            }
        }

        info!("ControllerHub instanced correctly!");
        Self::with_controllers(
            dispatcher_reciever,
            dispatcher_sender,
            local_c,
            spotify_c,
            music_player_c,
            auth_c,
        )
    }
}

impl<C: SpotifyClient> ControllerHub<C> {
    pub fn with_controllers(
        dispatcher_reciever: UnboundedReceiver<Action>,
        dispatcher_sender: UnboundedSender<Action>,
        local_controller: LocalController,
        spotify_controller: SpotifyController<C>,
        music_player_controller: MusicPlayerController,
        auth_controller: AuthController,
    ) -> Self {
        Self {
            reciever: dispatcher_reciever,
            sender: dispatcher_sender,
            local_controller,
            spotify_controller: Arc::new(RwLock::new(spotify_controller)),
            music_player_controller,
            auth_controller,
        }
    }

    pub async fn init_loop(mut self) {
        loop {
            tokio::select! {
                maybe_action = self.reciever.recv() => {
                    match maybe_action {
                        Some(action) => {
                            match action {
                                Action::Spotify(spotify_action) => self.handle_spotify_action(spotify_action).await,
                                Action::Local(local_action) => self.handle_local_actions(local_action),
                                Action::MusicPlayer(music_player_action) => self.handle_music_player_actions(music_player_action),
                                _ => {},
                            }
                        }
                        None => {
                            info!("Dispatcher channel closed; stopping ControllerHub loop.");
                            break;
                        }
                    }
                }
                audio_action = self.music_player_controller.recv_event() => {
                    if let Action::MusicPlayer(MusicPlayerAction::LibrespotInitialized(ref creds)) = audio_action {
                        let mut session = self.auth_controller.get_session();
                        session.librespot = Some(creds.clone());
                        self.auth_controller.save_session(session);
                    }
                    if !matches!(audio_action, Action::NoOp)
                        && let Err(e) = self.sender.send(audio_action)
                    {
                        error!("Error sending audio event action to dispatcher: {}", e);
                    }
                }
            }
        }
    }

    async fn sync_web_api_tokens(&mut self) {
        let current_web_api = {
            let sp = self.spotify_controller.read().await;
            sp.tokens()
        };
        self.auth_controller.sync_web_api(current_web_api);
    }

    pub async fn handle_spotify_action(&mut self, action: SpotifyAction) {
        let is_logout = matches!(action, SpotifyAction::LoggOut);
        let res_action = match action {
            SpotifyAction::LoggIn(client_id) => {
                let mut sp = self.spotify_controller.write().await;
                match sp.login(client_id.clone()).await {
                    Ok(_) => {
                        let mut session = self.auth_controller.get_session();
                        session.client_id = Some(client_id.clone());
                        session.web_api = sp.tokens();
                        self.auth_controller.save_session(session.clone());
                        self.music_player_controller
                            .login(&client_id, session.librespot);
                        Action::Spotify(SpotifyAction::LoggedIn)
                    }
                    Err(e) => Action::UnexpectedErrorHappened(e.to_string()),
                }
            }
            SpotifyAction::LoggOut => {
                let mut sp = self.spotify_controller.write().await;
                let act = sp.logout();
                self.music_player_controller.logout();
                self.auth_controller.erase_session();
                act
            }
            SpotifyAction::FetchPlaylists => {
                let sp = self.spotify_controller.read().await;
                sp.fetch_playlists().await
            }
            SpotifyAction::FetchAlbums => {
                let sp = self.spotify_controller.read().await;
                sp.fetch_albums().await
            }
            SpotifyAction::SearchQuery(query) => {
                let sp = self.spotify_controller.read().await;
                sp.search_query(&query).await
            }
            SpotifyAction::AddSongTo(playlist_id, song) => {
                let sp = self.spotify_controller.read().await;
                sp.add_song_to_playlist(&playlist_id, song).await
            }
            SpotifyAction::EraseSongFrom(playlist_id, song) => {
                let sp = self.spotify_controller.read().await;
                sp.remove_song_from_playlist(&playlist_id, song).await
            }
            SpotifyAction::CreatePlaylist(playlist_name, public, song) => {
                let sp = self.spotify_controller.read().await;
                sp.create_playlist(&playlist_name, public, song).await
            }
            SpotifyAction::DeletePlaylist(playlist_id) => {
                let sp = self.spotify_controller.read().await;
                sp.delete_playlist(&playlist_id).await
            }
            SpotifyAction::DeleteAlbum(album_id) => {
                let sp = self.spotify_controller.read().await;
                sp.delete_album(&album_id).await
            }
            _ => Action::NoOp,
        };

        if !is_logout {
            self.sync_web_api_tokens().await;
        }

        if !matches!(res_action, Action::NoOp)
            && let Err(e) = self.sender.send(res_action)
        {
            error!("Error happened sending a Spotify message: {}", e);
        }
    }

    pub fn handle_local_actions(&mut self, action: LocalAction) {
        let res_action = match action {
            LocalAction::FetchPlaylists => self.local_controller.fetch_playlists(),
            LocalAction::FetchSongs => self.local_controller.fetch_local_songs(),
            LocalAction::CreatePlaylist(playlist_name) => {
                self.local_controller.create_playlist(&playlist_name)
            }
            LocalAction::DeletePlaylist(playlist_id) => {
                self.local_controller.delete_playlist(playlist_id)
            }
            LocalAction::AddSongTo(song, playlist_id) => {
                self.local_controller.add_song_to_playlist(playlist_id, song)
            }
            LocalAction::EraseSongFrom(song_id, playlist_id) => self
                .local_controller
                .remove_song_from_playlist(playlist_id, song_id),
            _ => {
                return;
            }
        };

        if !matches!(res_action, Action::NoOp)
            && let Err(e) = self.sender.send(res_action)
        {
            error!("Error happened sending a message: {}", e);
        }
    }

    pub fn handle_music_player_actions(&mut self, action: MusicPlayerAction) {
        let res_action = match action {
            MusicPlayerAction::PlaySong(song) => {
                if song.is_local() && !self.local_controller.validate_file_existence(song.song_data())
                {
                    Action::UnexpectedErrorHappened(format!(
                        "The file for {} is no longer available on disk.",
                        song.song_name()
                    ))
                } else {
                    self.music_player_controller.play(song)
                }
            }
            MusicPlayerAction::PlayPlaylist(songs) => {
                self.music_player_controller.play_playlist(songs)
            }
            MusicPlayerAction::AddToQueue(song) => {
                if song.is_local() && !self.local_controller.validate_file_existence(song.song_data())
                {
                    Action::UnexpectedErrorHappened(format!(
                        "The file for {} is no longer available on disk.",
                        song.song_name()
                    ))
                } else {
                    self.music_player_controller.add_to_queue(song);
                    Action::NoOp
                }
            }
            MusicPlayerAction::EraseFromQueue(song_id) => {
                self.music_player_controller.erase_from_queue(&song_id);
                Action::NoOp
            }
            MusicPlayerAction::ClearQueue => {
                self.music_player_controller.clear_queue();
                Action::NoOp
            }
            MusicPlayerAction::TogglePause => {
                self.music_player_controller.toggle_pause();
                Action::NoOp
            }
            MusicPlayerAction::Seek(position) => {
                self.music_player_controller.seek(position);
                Action::NoOp
            }
            MusicPlayerAction::SkipToNext => self.music_player_controller.skip_to_next(),
            MusicPlayerAction::SkipToPrevious => self.music_player_controller.previous(),
            MusicPlayerAction::ChangeFlow(flow) => {
                self.music_player_controller.set_flow(&flow);
                Action::NoOp
            }
            _ => Action::NoOp,
        };

        if !matches!(res_action, Action::NoOp)
            && let Err(e) = self.sender.send(res_action)
        {
            error!("Error happened sending a music player message: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tokio::sync::mpsc;

    use super::*;
    use crate::controllers::test_doubles::{
        StubAlbum, StubAuth, StubPlaylist, StubSearch, StubUser,
    };
    use crate::domain::{
        audio_messages::{Event, Messages},
        auth_models::{LibrespotData, SessionData, WebApiData},
        spot_mp_errors::{SpotMpError, Status},
    };
    use crate::services::data_storage::auth_storage::AuthStorage;
    use crate::services::spotify_api::spotify_api_client::SpotifyApiClient;

    struct TempDirGuard {
        path: PathBuf,
    }

    impl TempDirGuard {
        fn new(name: &str) -> Self {
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

    type StubSpotifyApiClient =
        SpotifyApiClient<StubAuth, StubUser, StubPlaylist, StubAlbum, StubSearch>;

    struct TestHubHarness {
        hub: ControllerHub<StubSpotifyApiClient>,
        dispatched_rx: mpsc::UnboundedReceiver<Action>,
        audio_msg_rx: mpsc::UnboundedReceiver<Messages>,
        _audio_evt_tx: mpsc::UnboundedSender<Event>,
        _hub_in_tx: mpsc::UnboundedSender<Action>,
        temp_dir: TempDirGuard,
    }

    impl TestHubHarness {
        fn new(
            name: &str,
            initial_session: SessionData,
            auth: StubAuth,
            playlist: StubPlaylist,
            search: StubSearch,
        ) -> Self {
            let temp_dir = TempDirGuard::new(name);
            let storage = AuthStorage::new(temp_dir.path.clone());
            storage
                .save_session(initial_session.clone())
                .expect("seed initial session.json");

            let local_controller =
                LocalController::with_config_folder(temp_dir.path.clone()).expect("local ctrl");
            let auth_controller = AuthController::new(temp_dir.path.clone());

            let client =
                SpotifyApiClient::with_services(auth, StubUser, playlist, StubAlbum, search);
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

        fn read_session_from_disk(&self) -> SessionData {
            AuthStorage::new(self.temp_dir.path.clone())
                .load_session()
                .expect("load session.json from disk")
        }
    }

    fn seeded_session() -> SessionData {
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

    #[tokio::test]
    async fn silently_rotated_tokens_are_persisted_on_success_and_on_retried_5xx() {
        let auth = StubAuth::default();
        auth.push_renew_outcome(Ok(("access-v2".to_string(), "refresh-v2".to_string())));
        auth.push_renew_outcome(Ok(("access-v3".to_string(), "refresh-v3".to_string())));

        let playlist = StubPlaylist::with_responses(vec![
            Err(SpotMpError::SpotifyErrorStatus(Status::Unauthorized(
                "expired".to_string(),
            ))),
            Ok(vec![]),
        ]);
        let search = StubSearch::with_responses(vec![
            Err(SpotMpError::SpotifyErrorStatus(Status::Unauthorized(
                "expired".to_string(),
            ))),
            Err(SpotMpError::SpotifyErrorStatus(Status::Unknown(
                503,
                "service unavailable".to_string(),
            ))),
        ]);

        let initial = seeded_session();
        let mut harness = TestHubHarness::new(
            "rotated-tokens",
            initial.clone(),
            auth,
            playlist,
            search,
        );

        // 1. Silent rotation + retry succeeds -> persisted to session.json
        harness
            .hub
            .handle_spotify_action(SpotifyAction::FetchPlaylists)
            .await;

        let action = harness.dispatched_rx.try_recv().expect("dispatched action");
        assert!(matches!(
            action,
            Action::Spotify(SpotifyAction::FetchedPlaylists(_))
        ));

        let persisted_after_success = harness.read_session_from_disk();
        assert_eq!(persisted_after_success.client_id, initial.client_id);
        assert_eq!(persisted_after_success.librespot, initial.librespot);
        assert_eq!(
            persisted_after_success.web_api,
            Some(WebApiData {
                access_token: "access-v2".to_string(),
                refresh_token: "refresh-v2".to_string(),
            })
        );

        // 2. Silent rotation + retried call fails with 5xx -> rotated tokens STILL persisted!
        harness
            .hub
            .handle_spotify_action(SpotifyAction::SearchQuery("daft punk".to_string()))
            .await;

        let err_action = harness
            .dispatched_rx
            .try_recv()
            .expect("dispatched error action");
        assert!(matches!(err_action, Action::UnexpectedErrorHappened(_)));

        let persisted_after_5xx = harness.read_session_from_disk();
        assert_eq!(persisted_after_5xx.client_id, initial.client_id);
        assert_eq!(persisted_after_5xx.librespot, initial.librespot);
        assert_eq!(
            persisted_after_5xx.web_api,
            Some(WebApiData {
                access_token: "access-v3".to_string(),
                refresh_token: "refresh-v3".to_string(),
            })
        );
    }

    #[tokio::test]
    async fn session_expired_clears_only_web_api_and_keeps_client_id_and_librespot_active() {
        let auth = StubAuth::default();
        auth.push_renew_outcome(Err(SpotMpError::SpotifyErrorStatus(Status::BadRequest(
            "invalid_grant".to_string(),
        ))));

        let playlist = StubPlaylist::with_responses(vec![Err(SpotMpError::SpotifyErrorStatus(
            Status::Unauthorized("expired".to_string()),
        ))]);
        let search = StubSearch::with_responses(vec![]);

        let initial = seeded_session();
        let mut harness = TestHubHarness::new(
            "session-expired",
            initial.clone(),
            auth,
            playlist,
            search,
        );

        harness
            .hub
            .handle_spotify_action(SpotifyAction::FetchPlaylists)
            .await;

        let action = harness.dispatched_rx.try_recv().expect("dispatched action");
        assert!(
            matches!(action, Action::Spotify(SpotifyAction::SessionExpired)),
            "expected SessionExpired action, got: {action:?}"
        );

        // Only web_api is cleared on disk; client_id and librespot credentials remain intact.
        let persisted = harness.read_session_from_disk();
        assert_eq!(persisted.web_api, None);
        assert_eq!(persisted.client_id, initial.client_id);
        assert_eq!(persisted.librespot, initial.librespot);

        // Librespot playback was NOT logged out and still accepts playback commands.
        assert!(
            matches!(
                harness.audio_msg_rx.try_recv(),
                Err(mpsc::error::TryRecvError::Empty)
            ),
            "librespot must not be logged out on Web API SessionExpired"
        );
        harness
            .hub
            .handle_music_player_actions(MusicPlayerAction::TogglePause);
        assert!(matches!(
            harness.audio_msg_rx.try_recv(),
            Ok(Messages::TogglePause)
        ));
    }

    #[tokio::test]
    async fn explicit_logout_erases_full_session_and_logs_out_librespot() {
        let auth = StubAuth::default();
        let playlist = StubPlaylist::with_responses(vec![]);
        let search = StubSearch::with_responses(vec![]);

        let initial = seeded_session();
        let mut harness =
            TestHubHarness::new("explicit-logout", initial, auth, playlist, search);

        harness
            .hub
            .handle_spotify_action(SpotifyAction::LoggOut)
            .await;

        let action = harness.dispatched_rx.try_recv().expect("dispatched action");
        assert!(
            matches!(action, Action::Spotify(SpotifyAction::LoggedOut)),
            "expected LoggedOut action, got: {action:?}"
        );

        // Full session (client_id, web_api, librespot) is erased on disk.
        let persisted = harness.read_session_from_disk();
        assert_eq!(persisted, SessionData::default());

        // Librespot receives explicit logout command.
        assert!(matches!(
            harness.audio_msg_rx.try_recv(),
            Ok(Messages::LogoutLibrespot)
        ));
    }
}
