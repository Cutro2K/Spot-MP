use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use crate::state_store::app::action::Action;
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
    spotify_controller: SpotifyController,
    music_player_controller: MusicPlayerController,
    auth_controller: AuthController,
}

impl ControllerHub {

}
