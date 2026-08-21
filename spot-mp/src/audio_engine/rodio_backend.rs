use std::fs::File;
use std::time::Duration;
use crate::audio_engine::audio_backend::{AudioBackend};
use crate::domain::audio_messages::AudioSource;
use rodio::{Decoder, MixerDeviceSink, Player};
use crate::domain::spot_mp_errors::SpotMpError;

pub struct RodioBackend {
    _handle: MixerDeviceSink,
    player: Player,
    is_playing: bool,
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
            is_playing: false,
        }
    }
}

impl AudioBackend for RodioBackend {
    fn play(&mut self, source: AudioSource) -> Result<(), SpotMpError> {
        if let AudioSource::Local(path) = source {
            let file = File::open(path)?;
            let source = Decoder::try_from(file)?;
            self.player.clear();
            self.player.append(source);
            self.player.play();
            self.is_playing = true;
        }
        Ok(())
    }
    fn toggle_pause(&mut self) -> Result<(), SpotMpError> {
        if self.player.is_paused() {
            self.player.play();
            self.is_playing = true;
        } else {
            self.player.pause();
            self.is_playing = false;
        }
        Ok(())
    }
    fn seek(&mut self, position: u32) -> Result<(), SpotMpError> {
        self.player.try_seek(Duration::from_secs(position as u64))?;
        Ok(())
    }
    fn stop(&mut self) -> Result<(), SpotMpError> {
        if self.is_playing {
            self.player.stop();
            self.is_playing = false;
        }
        Ok(())
    }
    
    fn is_finished(&self) -> bool {
        self.player.empty()  
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    
    #[test]
    fn test_rodio_backend() {
        let mut rodio_backend = RodioBackend::new();
        
        println!("Playing a song with rodio...");
        rodio_backend.play(AudioSource::Local("./assets/songs/alexguz-funk-amp-breakbeat-541097.mp3".to_string())).expect("Failed to start playing a song with rodio backend!");
        
        println!("Reproducing audio for 2 seconds...");
        thread::sleep(Duration::from_secs(2));

        println!("Toggle pause...");
        rodio_backend.toggle_pause().unwrap();
        thread::sleep(Duration::from_secs(1));
        
        println!("Toggle pause again...");
        rodio_backend.toggle_pause().unwrap();
        
        println!("Reproducing audio for 2 seconds again...");
        thread::sleep(Duration::from_secs(2));

        println!("Stoping the player and waiting 2 more seconds...");
        rodio_backend.stop().unwrap();
        thread::sleep(Duration::from_secs(2));
    }
}
