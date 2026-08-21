use crate::domain::{auth_models::LibrespotData, spot_mp_errors::SpotMpError};

pub enum AudioSource {
    Local(String), // Song path
    Spotify(String), // Spotify id
}

pub enum Messages {
    Play(AudioSource),
    TogglePause,
    Seek(u32),
    LoginLibrespot(String, Option<LibrespotData>), // (client_id, Credentials)
    LogoutLibrespot,
}

pub enum ActiveBackend {
    Local,
    Spotify,
}

pub enum Event {
    SpotifyInitilized(LibrespotData), // Retrieve the last session credentials 
    Finished,
    Error(SpotMpError),
}
