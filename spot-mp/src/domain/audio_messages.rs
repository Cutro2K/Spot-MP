use crate::domain::spot_mp_errors::SpotMpError;

pub enum AudioSource {
    Local(String), // Song path
    Spotify(String), // Spotify id
}

pub enum Messages {
    Play(AudioSource),
    TogglePause,
    Seek(u32),
}

pub enum ActiveBackend {
    Local,
    Spotify,
}

pub enum Event {
    Finished,
    Error(SpotMpError),
}
