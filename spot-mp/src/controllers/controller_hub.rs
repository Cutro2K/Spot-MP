use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use crate::app::action::Action;
use crate::controllers::spotify_controller::SpotifyController;
use crate::controllers::local_controller::LocalController; 
use crate::controllers::music_player_controller::MusicPlayerController;
use crate::controllers::auth_controller::AuthController;

// ControllerHub comunicate whit dispatcher to handle backend calls and produce actions with changes
// to the frontend
#[allow(dead_code)]
pub struct ControllerHub {
    reciever: UnboundedReceiver<Action>,
    sender: UnboundedSender<Action>,
    local_controller: LocalController,
    spotify_controller: SpotifyController,
    music_player_controller: MusicPlayerController,
    auth_controller: AuthController,
}

impl ControllerHub {

}
