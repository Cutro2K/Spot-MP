use crate::domain::{auth_models::LibrespotData, spot_mp_errors::SpotMpError};

#[derive(Debug, Clone)]
pub enum AudioSource {
    Local(String),   // Song path
    Spotify(String), // Spotify id
}

#[derive(Debug, Clone)]
pub enum Messages {
    Play(AudioSource),
    TogglePause,
    Seek(u32),
    LoginLibrespot(String, Option<LibrespotData>), // (client_id, Credentials)
    LogoutLibrespot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveBackend {
    Local,
    Spotify,
}

#[derive(Debug)]
pub enum Event {
    SpotifyInitialized(LibrespotData), // Retrieve the last session credentials
    Finished,
    Error(SpotMpError),
}
