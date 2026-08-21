use crate::audio_engine::audio_backend::AudioBackend;
use crate::domain::audio_messages::AudioSource;
use crate::domain::auth_models::LibrespotData;
use crate::domain::spot_mp_errors::SpotMpError;
use librespot::core::{SpotifyId, SpotifyUri};
use librespot::discovery::{Credentials, DeviceType, Discovery};
use librespot::playback::config::{PlayerConfig, AudioFormat};
use librespot::playback::player::{Player, PlayerEvent};
use log::error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use librespot::core::{Session, SessionConfig};
use librespot::playback::{audio_backend, mixer};
use librespot::playback::mixer::{MixerConfig};
use futures::StreamExt;
use uuid::Uuid;

pub struct LibrespotBackend {
    player: Arc<Player>,
    is_playing: bool,
    is_finished: Arc<AtomicBool>,
}

impl LibrespotBackend {
    pub async fn new(client_id: &str) -> Result<(Self, LibrespotData), SpotMpError> {
        let device_name = "Spot-MP";
        let new_uuid = Uuid::new_v4();
        let device_id = format!("spot_mp_{}", new_uuid.simple());
                
        let mut discovery = Discovery::builder(device_id.clone(), client_id.to_string()).name(device_name).device_type(DeviceType::Speaker).launch()?;
        // Creating the session
        let session_config = SessionConfig::default();
        let session  = Session::new(session_config, None);
        let mut string_credentials = String::new();

        while let Some(new_credentials) = discovery.next().await {
            match session.connect(new_credentials.clone(), false).await {
                Ok(()) => {
                    string_credentials = serde_json::to_string(&new_credentials)?;
                    break;
                },
                Err(e) => {
                    error!("An error happend trying to connet with spotify: {}", e);
                }
            }
        }
    
        // Creating the player
        let player_config = PlayerConfig::default();
        let mixer_config = MixerConfig::default();
        let mixer_builder = mixer::find(None).ok_or_else(|| SpotMpError::LibrespotInitialization("Couldn't find a mixer for librespot!".to_string()))?;
        let mixer = mixer_builder(mixer_config)?;        
        let backend_fn = audio_backend::find(Some("rodio".to_string())).ok_or_else(|| SpotMpError::LibrespotInitialization("Couldn't find rodio backend!".to_string()))?;
        let sink_builder = move || backend_fn(None, AudioFormat::default());
        
        let new_player = Player::new(
            player_config,
            session,
            mixer.get_soft_volume(),
            sink_builder
        );
        
        // Getting the is finished arch with the event_channel
        let is_finished = Arc::new(AtomicBool::new(false));
        let is_finished_clone = Arc::clone(&is_finished);
        let mut event_channel = new_player.get_player_event_channel();

        tokio::spawn(async move {
            while let Some(event) = event_channel.recv().await {
                match event {
                    PlayerEvent::EndOfTrack { .. } => {
                        is_finished_clone.store(true, Ordering::SeqCst);
                    },
                    PlayerEvent::Playing { .. } => {
                        is_finished_clone.store(false, Ordering::SeqCst);
                    },
                    _ => {}
                }
            }
        })  ;

        Ok((
            Self {
                player: new_player,
                is_playing: false,
                is_finished,
            },
            LibrespotData {
                device_id: device_id.to_string(),
                credentials: string_credentials,
            }
        ))
    }
    
    // This is an async function but doesn't require user interaction
    pub async fn new_with_credentials(client_id: &str, data: LibrespotData) -> Result<(Self, LibrespotData), SpotMpError> {
        let credentials: Credentials = serde_json::from_str(&data.credentials)?;

        let session = Session::new(SessionConfig::default(), None);
        let player_config = PlayerConfig::default();
        
        let mixer_config = MixerConfig::default();
        let mixer_builder = mixer::find(None).ok_or_else(|| SpotMpError::LibrespotInitialization("Couldn't find a mixer for librespot!".to_string()))?;
        let mixer = mixer_builder(mixer_config)?;
        
        let backend_fn = audio_backend::find(Some("rodio".to_string())).ok_or_else(|| SpotMpError::LibrespotInitialization("Couldn't find rodio backend!".to_string()))?;
        let sink_builder = move || backend_fn(None, AudioFormat::default());      
        
        

        match session.connect(credentials, false).await {
            Ok(_) => {
                let player = Player::new( 
                    player_config, 
                    session, 
                    mixer.get_soft_volume(), 
                    sink_builder
                );
                // Getting the is finished arch with the event_channel
                let is_finished = Arc::new(AtomicBool::new(false));
                let is_finished_clone = Arc::clone(&is_finished);
                let mut event_channel = player.get_player_event_channel();

                tokio::spawn(async move {
                    while let Some(event) = event_channel.recv().await {
                        match event {
                            PlayerEvent::EndOfTrack { .. } => {
                                is_finished_clone.store(true, Ordering::SeqCst);
                            },
                            PlayerEvent::Playing { .. } => {
                                is_finished_clone.store(false, Ordering::SeqCst);
                            },
                            _ => {}
                        }
                    }
                })  ;

                Ok((
                        Self {
                            player,
                            is_playing: false,
                            is_finished,
                        },
                        data,
                ))
            }
            Err(_) => {
                Self::new(client_id).await
            }
        }
    }
}

impl AudioBackend for LibrespotBackend {
    fn play(&mut self, source: AudioSource) -> Result<(), SpotMpError> {
        if let AudioSource::Spotify(spotify_id) = source {            
            self.player.load(SpotifyUri::Track { id: SpotifyId::from_base62(&spotify_id)? }, true, 0);
            self.player.play();
            self.is_playing = true;
        }
        Ok(())
    }
    fn toggle_pause(&mut self) -> Result<(), SpotMpError> {
        if self.is_playing {
            self.player.pause();
            self.is_playing = false;
        }
        else {
            self.player.play();
            self.is_playing = true;
        }
        Ok(())
    }
    fn seek(&mut self, position: u32) -> Result<(), SpotMpError> {
        self.player.seek(position);
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
        self.is_finished.load(Ordering::SeqCst)
    }
}
