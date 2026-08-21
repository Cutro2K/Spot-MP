use std::sync::mpsc::{Receiver, Sender};
use crate::app::action::Action;
use crate::controllers::spotify_controller::SpotifyController;
use crate::controllers::local_controller::LocalController; 
use crate::controllers::music_player_controller::MusicPlayerController;
use crate::controllers::auth_controller::AuthController;

// ControllerHub comunicate whit dispatcher to handle backend calls and produce actions with changes
// to the frontend

pub struct ControllerHub {
    reciever: Receiver<Action>,
    sender: Sender<Action>,
    local_controller: LocalController,
    spotify_controller: SpotifyController,
    music_player_controller: MusicPlayerController,
    auth_controller: AuthController,
}

impl ControllerHub {

}
