use crate::domain::{audio_messages::AudioSource, spot_mp_errors::SpotMpError};

pub trait AudioBackend {
    fn play(&mut self, source: AudioSource) -> Result<(), SpotMpError>;
    fn toggle_pause(&mut self) -> Result<(), SpotMpError>;
    fn seek(&mut self, position: u32) -> Result<(), SpotMpError>;
    fn stop(&mut self) -> Result<(), SpotMpError>;
}
