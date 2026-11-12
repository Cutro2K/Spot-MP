use log::{error, info};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::controllers::auth_controller::AuthController;
use crate::controllers::local_controller::LocalController;
use crate::controllers::music_player_controller::MusicPlayerController;
use crate::controllers::spotify_controller::SpotifyController;
use crate::state_store::app::action::{Action, LocalAction, MusicPlayerAction, SpotifyAction};

// ControllerHub communicates with dispatcher to handle backend calls and produce actions with changes
// to the frontend; some functions will execute synchronously and others asynchronously depending on
// whether the function has to mutate the internal values of the attributes.

#[allow(dead_code)]
pub struct ControllerHub {
    reciever: UnboundedReceiver<Action>, // Comes from the dispatcher
    sender: UnboundedSender<Action>,     // Goes to the dispatcher
    local_controller: LocalController,
    spotify_controller: Arc<RwLock<SpotifyController>>,
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
        Self {
            reciever: dispatcher_reciever,
            sender: dispatcher_sender,
            local_controller: local_c,
            spotify_controller: Arc::new(RwLock::new(spotify_c)),
            music_player_controller: music_player_c,
            auth_controller: auth_c,
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

    pub async fn handle_spotify_action(&mut self, action: SpotifyAction) {
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
            SpotifyAction::RenewToken => {
                let mut sp = self.spotify_controller.write().await;
                match sp.renew_token().await {
                    Ok(_) => {
                        let mut session = self.auth_controller.get_session();
                        session.web_api = sp.tokens();
                        self.auth_controller.save_session(session);
                        Action::Spotify(SpotifyAction::LoggedIn)
                    }
                    Err(e) => Action::UnexpectedErrorHappened(e.to_string()),
                }
            }
            SpotifyAction::FetchPlaylists => {
                let mut sp = self.spotify_controller.write().await;
                sp.fetch_playlists().await
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
            _ => Action::NoOp,
        };

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
            MusicPlayerAction::PlaySong(song) => self.music_player_controller.play(song),
            MusicPlayerAction::PlayPlaylist(songs) => {
                self.music_player_controller.play_playlist(songs)
            }
            MusicPlayerAction::AddToQueue(song) => {
                self.music_player_controller.add_to_queue(song);
                Action::NoOp
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
