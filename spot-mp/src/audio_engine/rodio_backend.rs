use std::fs::File;
use std::time::Duration;
use crate::audio_engine::audio_backend::{AudioBackend};
use crate::domain::audio_messages::AudioSource;
use rodio::{Decoder, MixerDeviceSink, Player};

pub struct RodioBackend {
    _handle: MixerDeviceSink,
    player: Player,
}

impl Default for RodioBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RodioBackend {
    pub fn new() -> Self {
        let handler = rodio::DeviceSinkBuilder::open_default_sink().expect("Can't open the default sink for audio streaming with rodio!");
        Self {
            player: Player::connect_new(handler.mixer()),
            _handle: handler,
        }
    }
}

impl AudioBackend for RodioBackend {
    fn play(&mut self, source: AudioSource) -> Result<(), crate::domain::spot_mp_errors::SpotMpError> {
        if let AudioSource::Local(path) = source {
            let file = File::open(path)?;
            let source = Decoder::try_from(file)?;
            self.player.clear();
            self.player.append(source);
            self.player.play();
        }
        Ok(())
    }
    fn toggle_pause(&mut self) -> Result<(), crate::domain::spot_mp_errors::SpotMpError> {
        if self.player.is_paused() {
            self.player.play();
        } else {
            self.player.pause();
        }
        Ok(())
    }
    fn seek(&mut self, position: u32) -> Result<(), crate::domain::spot_mp_errors::SpotMpError> {
        self.player.try_seek(Duration::from_secs(position as u64))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    
    #[test]
    #[ignore]
    fn test_play_song_rodio_backend() {
        let mut rodio_backend = RodioBackend::new();
        
        rodio_backend.play(AudioSource::Local("./assets/songs/alexguz-funk-amp-breakbeat-541097.mp3".to_string())).expect("Failed to start playing a song with rodio backend!");
        
        rodio_backend.seek(30).expect("Error while trying seek at 30 playing whit rodio backend");

        println!("Reproducing audio for 5 seconds...");
        thread::sleep(Duration::from_secs(2));
        let _ = rodio_backend.toggle_pause();
        thread::sleep(Duration::from_secs(1));
        let _ = rodio_backend.toggle_pause();
        thread::sleep(Duration::from_secs(2));
    }
}
