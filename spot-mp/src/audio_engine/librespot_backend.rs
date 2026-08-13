use crate::audio_engine::audio_backend::AudioBackend;
use crate::domain::audio_messages::AudioSource;
use librespot::core::{SpotifyId, SpotifyUri};
use librespot::playback::config::{PlayerConfig, AudioFormat};
use librespot::playback::player::Player;
use std::sync::Arc;
use librespot::core::{Session, SessionConfig, authentication::Credentials};
use librespot::playback::audio_backend;
use librespot::playback::mixer::{NoOpVolume, VolumeGetter};

pub struct LibrespotBackend {
    player: Arc<Player>,
}

impl LibrespotBackend {
    pub async fn new(access_token: String) -> Self {
        let config = PlayerConfig::default();

        let volume_getter: Box<dyn VolumeGetter + Send> = Box::new(NoOpVolume);
        
        let backend_fn = audio_backend::find(None).unwrap();

        let sink_builder = move || backend_fn(None, AudioFormat::default());
        
        let session = Session::new(SessionConfig::default(), None);
        
        session.connect(Credentials::with_access_token(access_token), false).await.expect("Can't connect librespot with access token!");

        let new_player = Player::new(
            config,
            session,
            volume_getter,
            sink_builder
        );

        Self {
            player: new_player,
        }
    }
}

impl AudioBackend for LibrespotBackend {
    fn play(&mut self, source: AudioSource) -> Result<(), crate::domain::spot_mp_errors::SpotMpError> {
        if let AudioSource::Spotify(spotify_id) = source {
            self.player.load(SpotifyUri::Track { id: SpotifyId::from_base62(&spotify_id)? }, true, 0);
        }
        Ok(())
    }
    fn toggle_pause(&mut self) -> Result<(), crate::domain::spot_mp_errors::SpotMpError> {
        self.player.pause();
        Ok(())
    }
    fn seek(&mut self, position: u32) -> Result<(), crate::domain::spot_mp_errors::SpotMpError> {
        self.player.seek(position);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use crate::spotify_api::spotify_api_client::SpotifyApiClient; 
    use crate::domain::audio_messages::AudioSource; 

    #[tokio::test]
    async fn test_librespot_backend() {
        let mut spotify_client = SpotifyApiClient::new(); 
        let my_real_client_id = "850a0abcc2474ff5aad399aeb9489d06".to_string();

        spotify_client.login(my_real_client_id).await.expect("Failed to login!");

        let access_token = spotify_client.access_token().clone().expect("The spotify client doesn't save the access token value!");
        
        println!("{}", access_token);

        let mut backend = LibrespotBackend::new(access_token).await;

        let track_id = "4cOdK2wGLETKBW3PvgPWqT".to_string(); // Never Gonna Give You Up
        let source = AudioSource::Spotify(track_id);

        let result = backend.play(source);
        assert!(result.is_ok(), "The play function failed with librespot backend: {:?}", result.err());

        tokio::time::sleep(Duration::from_secs(5)).await;

        let pause_result = backend.toggle_pause();
        assert!(pause_result.is_ok());

        tokio::time::sleep(Duration::from_secs(2)).await;
    }}
