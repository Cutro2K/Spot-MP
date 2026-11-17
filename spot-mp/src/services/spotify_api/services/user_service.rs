use crate::{domain::spot_mp_errors::SpotMpError, services::spotify_api::{client::http_client::SpotifyHttpClient, models::UserProfileResponse, traits::UserManager}};

pub struct UserService {
    id: Option<String>,
    http: SpotifyHttpClient,
}

impl UserService {
    pub fn new(http: SpotifyHttpClient) -> Self{
        Self {
            id: None,
            http: http,
        }
    }

    pub fn user_id(&self) -> Option<&str> {
        self.id.as_deref()
    }
}

impl UserManager for UserService {
    async fn fetch_current_user_id(
        &mut self
    ) -> Result<String, SpotMpError>
    {   
        if let Some(uid) = self.user_id() {
            return Ok(uid.to_string());
        }
        let url = "https://api.spotify.com/v1/me";

        let response = self.http.get(url).send_json::<UserProfileResponse>().await?;

        self.id = Some(response.id.clone());
        Ok(response.id)
    }
    
    fn clear_user_id(
        &mut self
    )
    {
        self.id = None;
    }
}