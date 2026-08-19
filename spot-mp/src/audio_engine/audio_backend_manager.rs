use crate::audio_engine::audio_backend::AudioBackend;
use crate::audio_engine::{librespot_backend::LibrespotBackend, rodio_backend::RodioBackend};
use crate::domain::audio_messages::{Messages, Event, ActiveBackend};
use crate::domain::auth_models::LibrespotData;
use std::sync::mpsc::{Receiver, Sender};
use crate::domain::audio_messages::AudioSource;
use crate::domain::spot_mp_errors::SpotMpError;

#[allow(dead_code)]
pub struct AudioBackendManager {
    current_backend: Option<ActiveBackend>,
    spotify_backend: Option<LibrespotBackend>,
    rodio_backend: RodioBackend,
    receiver: Receiver<Messages>,
    sender: Sender<Event>,
}

impl AudioBackendManager {
    pub fn new(receiver: Receiver<Messages>, sender: Sender<Event>) -> Self {
        Self {
            current_backend: None,
            spotify_backend: None,
            rodio_backend: RodioBackend::new(),
            receiver,
            sender,
        }
    }
    pub async fn login_librespot(&mut self, client_id: &str, data: Option<LibrespotData>) -> Result<LibrespotData, SpotMpError> {
        if let Some(login_data) = data {
            let (spot_backend, new_data) = LibrespotBackend::new_with_credentials(client_id, login_data).await?;
            self.spotify_backend = Some(spot_backend);
            Ok(new_data)
        } 
        else {
            let (spot_backend, new_data) = LibrespotBackend::new(client_id).await?;
            self.spotify_backend = Some(spot_backend);
            Ok(new_data)
        }
    }
    pub fn logout_librespot(&mut self) {
        self.spotify_backend.take();
    }
}

impl AudioBackend for AudioBackendManager {
    fn play(&mut self, source: AudioSource) -> Result<(), SpotMpError> {
        match source {
            AudioSource::Local(song_path) => {
                if let Some(ActiveBackend::Spotify) = self.current_backend && let Some(backend) = self.spotify_backend.as_mut() {
                    backend.stop()?;
                }
                self.rodio_backend.play(AudioSource::Local(song_path))?;
                self.current_backend = Some(ActiveBackend::Local);
            },
            AudioSource::Spotify(song_id) => {
                if self.spotify_backend.is_none() {
                    return Err(SpotMpError::UninitializedLibrespot());
                }
                if let Some(ActiveBackend::Local) = self.current_backend {
                    self.rodio_backend.stop()?;
                }
                self.spotify_backend.as_mut().unwrap().play(AudioSource::Spotify(song_id))?;
                self.current_backend = Some(ActiveBackend::Spotify);
            },
        }
        Ok(()) 
    }
    fn toggle_pause(&mut self) -> Result<(), SpotMpError> {
        match self.current_backend {
            Some(ActiveBackend::Local) => {
                self.rodio_backend.toggle_pause()?;
            },
            Some(ActiveBackend::Spotify) => {
                if self.spotify_backend.is_none() {
                    return Err(SpotMpError::UninitializedLibrespot());
                }
                self.spotify_backend.as_mut().unwrap().toggle_pause()?;
            },
            None => {
                return Err(SpotMpError::UnknownAudioBackend("Tried to toggle pause but the audio backend isn't initialized!".to_string()))  
            },
        }
        Ok(())
    }
    fn seek(&mut self, position: u32) -> Result<(), SpotMpError> {
        match self.current_backend {
            Some(ActiveBackend::Local) => {
                self.rodio_backend.seek(position)?;
            },
            Some(ActiveBackend::Spotify) => {
                if self.spotify_backend.is_none() {
                    return Err(SpotMpError::UninitializedLibrespot());
                }
                self.spotify_backend.as_mut().unwrap().seek(position)?;
            },
            None => {
                return Err(SpotMpError::UnknownAudioBackend("Tried to seek an specific point of the song but the audio backend isn't initialized!".to_string()))  
            },
        }
        Ok(())
    }
    fn stop(&mut self) -> Result<(), SpotMpError> {
        match self.current_backend {
            Some(ActiveBackend::Local) => {
                self.rodio_backend.stop()?;
            },
            Some(ActiveBackend::Spotify) => {
                if self.spotify_backend.is_none() {
                    return Err(SpotMpError::UninitializedLibrespot());
                }
                self.spotify_backend.as_mut().unwrap().stop()?;
            },
            None => {
                return Err(SpotMpError::UnknownAudioBackend("Tried to seek an specific point of the song but the audio backend isn't initialized!".to_string()))  
            },
        }
        self.current_backend = None;
        Ok(())
    }
}
