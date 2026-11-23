use std::sync::RwLock;

use crate::{domain::spot_mp_errors::SpotMpError, services::spotify_api::{client::http_client::SpotifyHttpClient, models::UserProfileResponse, traits::UserManager}};

pub struct UserService {
    id: RwLock<Option<String>>,
    http: SpotifyHttpClient,
}

impl UserService {
    pub fn new(http: SpotifyHttpClient) -> Self {
        Self {
            id: RwLock::new(None),
            http,
        }
    }

    pub fn user_id(&self) -> Option<String> {
        self.id.read().unwrap().clone()
    }
}

impl UserManager for UserService {
    async fn fetch_current_user_id(&self) -> Result<String, SpotMpError> {
        if let Some(uid) = self.user_id() {
            return Ok(uid);
        }
        let url = self.http.api_url("/v1/me");

        let response = self.http.get(&url).send_json::<UserProfileResponse>().await?;

        *self.id.write().unwrap() = Some(response.id.clone());
        Ok(response.id)
    }

    fn clear_user_id(&self) {
        *self.id.write().unwrap() = None;
    }
}