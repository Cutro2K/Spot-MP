use log::{error, info};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::state_store::app::action::{Action, LocalAction, MusicPlayerAction, SpotifyAction};
use crate::controllers::spotify_controller::SpotifyController;
use crate::controllers::local_controller::LocalController; 
use crate::controllers::music_player_controller::MusicPlayerController;
use crate::controllers::auth_controller::AuthController;

// ControllerHub comunicate whit dispatcher to handle backend calls and produce actions with changes
// to the frontend, some functions will excecute synchronously and other synchronously depending if
// the function has to mutate the internal values of the atributes.

// If the functions doesn't need to mutate the values the controller hub excecute that functions
// synchronously cloning the sender and using it itself

#[allow(dead_code)]
pub struct ControllerHub {
    reciever: UnboundedReceiver<Action>, // Comes of the dispatcher
    sender: UnboundedSender<Action>, // Goes to the dispatcher
    local_controller: LocalController,
    spotify_controller: Arc<RwLock<SpotifyController>>,
    music_player_controller: MusicPlayerController,
    auth_controller: AuthController,
}

impl ControllerHub {
    pub fn new(
        dispatcher_reciever: UnboundedReceiver<Action>,
        dispatcher_sender: UnboundedSender<Action>
        ) -> Self {
        
        let local_c = LocalController::new().unwrap_or_else(|e| {
            error!("Instancing ControllerHub class(local controller): {}", e);
            panic!("Failed Instancing ControllerHub class!");
        });
        let spotify_c = SpotifyController::new();
        let music_player_c = MusicPlayerController::new();
        let auth_c = AuthController::new(local_c.get_spot_mp_folder());

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
            match self.reciever.try_recv() {
                // Action Handler
                Ok(action) => {
                    match action {
                        Action::Spotify(spotify_action) => self.handle_spotify_action(spotify_action).await,
                        Action::Local(local_action) => self.handle_local_actions(local_action),
                        Action::MusicPlayer(music_player_action) => self.handle_music_player_actions(music_player_action),
                        _ => {},
                        
                    }  
                },
                Err(e) => {
                    error!("{}", e);
                }
            }
        }
    }

    pub async fn handle_spotify_action(&mut self, _action: SpotifyAction) {
    }
    pub fn handle_local_actions(&mut self, action: LocalAction) {
        let res_action = match action {
            LocalAction::FetchPlaylists => {
                self.local_controller.fetch_playlists()
            },
            LocalAction::FetchSongs => {
                self.local_controller.fetch_local_songs()
            },
            LocalAction::CreatePlaylist(playlist_name) => {
                self.local_controller.create_playlist(&playlist_name)
            },
            LocalAction::DeletePlaylist(playlist_id) => {
                self.local_controller.delete_playlist(playlist_id)
            },
            LocalAction::AddSongTo(song, playlist_id) => {
                self.local_controller.add_song_to_playlist(playlist_id, song)
            },
            LocalAction::EraseSongFrom(song_id, playlist_id) => {
                self.local_controller.remove_song_from_playlist(playlist_id, song_id)
            },
            _ => {
                return;
            },
        };
        
        if let Err(e) = self.sender.send(res_action) {
            error!("Error happened sending a message: {}", e);
        }
    }
    pub fn handle_music_player_actions(&self, action: MusicPlayerAction) {
        let res_action = match action {
            _ => {
                return;
            },
        };
    }
}
