use crate::services::audio_engine::audio_backend::AudioBackend;
use crate::services::audio_engine::{librespot_backend::LibrespotBackend, rodio_backend::RodioBackend};
use crate::domain::audio_messages::{Messages, Event, ActiveBackend};
use crate::domain::auth_models::LibrespotData;
use log::error;
use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver};
use tokio::time::timeout;
use std::time::Duration;
use crate::domain::audio_messages::AudioSource;
use crate::domain::spot_mp_errors::SpotMpError;

pub struct AudioBackendManager {
    current_backend: Option<ActiveBackend>,
    spotify_backend: Option<LibrespotBackend>,
    rodio_backend: RodioBackend,
    receiver: UnboundedReceiver<Messages>,
    sender: UnboundedSender<Event>,
}

impl AudioBackendManager {
    pub fn new(receiver: UnboundedReceiver<Messages>, sender: UnboundedSender<Event>) -> Self {
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

    // Audio Engine Primary Loop
    pub fn start_engine(mut self) {

        tokio::spawn(async move {
            let mut is_playing = false;

            loop {
                // Check if we receive a message and do something
                match timeout(Duration::from_millis(100), self.receiver.recv()).await {
                    Ok(Some(message)) => match message {
                        Messages::Play(source) => {
                            match self.play(source) {
                                Ok(_) => {
                                    is_playing = true;
                                },
                                Err(e) => {
                                    let _ = self.sender.send(Event::Error(e));
                                    is_playing = false;
                                }
                            }
                        },
                        Messages::TogglePause => {
                            if self.toggle_pause().is_ok() {
                                is_playing = !is_playing;
                            }
                        },
                        Messages::Seek(position) => {
                            if let Err(e) = self.seek(position) {
                                error!("{}", e);
                            }
                        },
                        Messages::LoginLibrespot(client_id, cretendials) => {
                            match self.login_librespot(&client_id, cretendials).await {
                                Ok(data) => {
                                    if let Err(e) = self.sender.send(Event::SpotifyInitilized(data)) {
                                        error!("{}", e);
                                    }
                                },
                                Err(e) => {
                                    let _ = self.sender.send(Event::Error(e));
                                }
                            }
                        },
                        Messages::LogoutLibrespot => {
                            self.logout_librespot();
                        }
                    },
                    Ok(None) => {
                        break;
                    }
                    Err(_) => {
                        if is_playing && self.is_finished() {
                            is_playing = false;
                            if let Err(e) = self.stop() {
                                error!("{}", e);
                            }
                            if let Err(e) = self.sender.send(Event::Finished) {
                                error!("{}", e);
                            }
                        }
                    }
                }
            }
        });
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

    fn is_finished(&self) -> bool {
        match self.current_backend {
            Some(ActiveBackend::Local) => {
                self.rodio_backend.is_finished()
            },
            Some(ActiveBackend::Spotify) => {
                if let Some(spotify_back) = &self.spotify_backend {
                    spotify_back.is_finished()
                }
                else {
                    true
                }
            },
            None => {
                true
            }
        }
    }
}
