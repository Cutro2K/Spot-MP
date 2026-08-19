use crate::audio_engine::audio_backend::AudioBackend;
use crate::domain::audio_messages::AudioSource;
use crate::domain::auth_models::LibrespotData;
use crate::domain::spot_mp_errors::SpotMpError;
use librespot::core::{SpotifyId, SpotifyUri};
use librespot::discovery::{Credentials, DeviceType, Discovery};
use librespot::playback::config::{PlayerConfig, AudioFormat};
use librespot::playback::player::{Player};
use std::sync::Arc;
use librespot::core::{Session, SessionConfig};
use librespot::playback::{audio_backend, mixer};
use librespot::playback::mixer::{MixerConfig};
use futures::StreamExt;
use uuid::Uuid;

pub struct LibrespotBackend {
    player: Arc<Player>,
    is_playing: bool,
}

impl LibrespotBackend {
    pub async fn new(client_id: &str) -> Result<(Self, LibrespotData), SpotMpError> {
        let device_name = "Spot-MP";
        let new_uuid = Uuid::new_v4();
        let device_id = format!("spot_mp_{}", new_uuid.simple());

        let mut discovery = Discovery::builder(device_id.clone(), client_id.to_string()).name(device_name).device_type(DeviceType::Speaker).launch()?;

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
                   eprint!("An error happend trying to connet with spotify: {}", e);
                }
            }
        }
    
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
 
        Ok((
            Self {
                player: new_player,
                is_playing: false,
            },
            LibrespotData {
                device_id: device_id.to_string(),
                credentials: string_credentials,
            }
        ))
    }

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
                Ok((
                        Self {
                            player,
                            is_playing: false,
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
}

#[cfg(test)]
mod tests {
    use tokio::time::{sleep, Duration};
    use super::*;

    #[tokio::test]
    async fn test_librespot_backend() {
        let my_real_client_id = "850a0abcc2474ff5aad399aeb9489d06";
        
        println!("Go to spotify and click in your device to activate the spotify connection!");
        let (mut librespot_player, _) = LibrespotBackend::new(my_real_client_id).await.expect("Failed to start librespot backend!");
            
        let never_gonna_give_you_up = "4PTG3Z6ehGkBFwjybzWkR8";

        librespot_player.play(AudioSource::Spotify(never_gonna_give_you_up.to_string())).expect("Failed to play never gonna give you up with librespot backend!");
        // librespot_player.player.await_end_of_track().await; If I uncomment this line works.
        
        println!("Reproducing audio for 10 seconds...");
        sleep(Duration::from_secs(10)).await;
        
        println!("Seeking to 70 secods and waiting 10 second...");
        librespot_player.seek(70).expect("Failed to seek!");
        sleep(Duration::from_secs(10)).await;

        println!("Toggle pause and waiting 10 second...");
        librespot_player.toggle_pause().expect("Failed to toggle pause!");
        sleep(Duration::from_secs(10)).await;

        println!("Toggle pause and reproduce for 20 seconds...");
        librespot_player.toggle_pause().expect("Failed to toggle pause!");
        sleep(Duration::from_secs(20)).await;

        println!("Stoping the reproductor and waiting 10 seconds...");
        librespot_player.stop().expect("Failed to stop!");
        sleep(Duration::from_secs(10)).await;
    }
}
