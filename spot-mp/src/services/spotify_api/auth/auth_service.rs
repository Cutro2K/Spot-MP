use std::{collections::HashMap};

use crate::{
    domain::spot_mp_errors::SpotMpError, services::spotify_api::{
        auth::{ 
            browser_opener::SystemBrowserOpener, callback_server::AuthCallbackServer, oauth_config::OAuthConfig, pkce::SpotifyAuth}, client::http_client::SpotifyHttpClient, models::SpotifyTokenResponse, traits::{AuthManager, BrowserOpener}
    }
};


const SPOTIFY_AUTH_URL: &str = "https://accounts.spotify.com/authorize";
const SPOTIFY_TOKEN_URL: &str = "https://accounts.spotify.com/api/token";



pub struct AuthService<B: BrowserOpener = SystemBrowserOpener> {
    http: SpotifyHttpClient,
    browser: B,
    oauth_config: OAuthConfig,
}

impl AuthService<SystemBrowserOpener> {
    pub fn new(http: SpotifyHttpClient, oauth_config: OAuthConfig) -> Self {
        Self {
            http,
            browser: SystemBrowserOpener,
            oauth_config,
        }
    }
}

impl<B: BrowserOpener> AuthService<B> {
    /// Constructor pensado para tests o implementaciones alternativas (ej. SSH / headless)
    pub fn with_browser(http: SpotifyHttpClient, oauth_config: OAuthConfig, browser: B) -> Self {
        Self {
            http,
            browser,
            oauth_config,
        }
    }
}

impl<B: BrowserOpener> AuthManager for AuthService<B> {
    async fn login(
        &mut self,
        client_id: &str
    ) -> Result<(), SpotMpError> {
        // Defining Spotify Scopes (Permisions for the API)
        const SPOTIFY_SCOPES: &str = concat!(
            "playlist-read-private playlist-read-collaborative ",
            "playlist-modify-private playlist-modify-public ",
            "user-modify-playback-state user-read-playback-state ",
            "user-library-read user-library-modify user-read-currently-playing ",
            "user-read-email user-read-private"
        );

        // Generating the OAuth code verifier and challenge
        let auth_code = SpotifyAuth::new();
        let code_verifier = auth_code.code_verifier();
        let code_challenge = auth_code.code_challenge();

        // Generating the REDIRECT_URI      
        let redirect_uri = self.oauth_config.redirect_uri();

        // Parameters of the endpoint
        let mut auth_params = HashMap::new();
        auth_params.insert("client_id", client_id);
        auth_params.insert("response_type", "code");
        auth_params.insert("redirect_uri", &redirect_uri);
        auth_params.insert("scope", SPOTIFY_SCOPES);
        auth_params.insert("code_challenge_method", "S256");
        auth_params.insert("code_challenge", code_challenge);

        // Setting up the url for the http call
        let url = reqwest::Url::parse_with_params(
            SPOTIFY_AUTH_URL, 
            auth_params
        )?;
        

        // Opening the browseer for login session
        if let Err(e) = self.browser.open(url.as_str()) {
            log::error!("Error opening browser: {}. Open this URL manually: {}", e, url);
        }

        // Listen for code verifier
        let bind_addr = self.oauth_config.bind_addr();
        let code = AuthCallbackServer::listen_for_code(&bind_addr).await?;
        

        // Third Step: Requesting an access & refresh token:
        let mut params = HashMap::new();
        params.insert("client_id", client_id);
        params.insert("grant_type", "authorization_code");
        params.insert("redirect_uri", &redirect_uri);
        params.insert("code", &code);
        params.insert("code_verifier", code_verifier);
        
        // Making an http call to get the token data.
        let token_data = self
            .http
            .post(SPOTIFY_TOKEN_URL)
            .without_auth()
            .form(&params)
            .send_json::<SpotifyTokenResponse>()
            .await?;

        // Calculating the expires_at variable for the http_client
        let valid_secs = token_data.expires_in.saturating_sub(60).max(0) as u64;
        let expires_at = std::time::Instant::now() + std::time::Duration::from_secs(valid_secs);

        // Validating refresh_token
        let refresh_token = token_data.refresh_token.ok_or_else(|| {
            SpotMpError::Error("Cannot retrieve the refresh token value after login!".to_string())
        })?;
       
        self.http
            .set_tokens(
                Some(token_data.access_token),
                Some(refresh_token),
                Some(expires_at),
            );
        Ok(())
    }

    fn logout(&mut self){ 
        self.http.set_tokens(
            None, 
            None, 
            None
        );
    }

    async fn renew_token(
        &mut self, 
        client_id: &str
    ) -> Result<(), SpotMpError>
    {
        // Getting the refresh token
        let mut refresh_token = self.http.require_refresh()?;

        let mut params = HashMap::new();
        params.insert("grant_type", "refresh_token");
        params.insert("refresh_token", &refresh_token);
        params.insert("client_id", client_id);
        
        let token_data = self.http
            .post(SPOTIFY_TOKEN_URL)
            .without_auth()
            .form(&params).send_json::<SpotifyTokenResponse>()
            .await?;

        let valid_secs = token_data.expires_in.saturating_sub(60).max(0) as u64;
        let expires_at = std::time::Instant::now() + std::time::Duration::from_secs(valid_secs);
        
        if let Some(new_refresh_token) = token_data.refresh_token {
            refresh_token = new_refresh_token;
        }

        self.http.set_tokens(Some(token_data.access_token), Some(refresh_token), Some(expires_at));
        Ok(())
    }

    fn set_tokens(
        &mut self, 
        access_token: Option<String>, 
        refresh_token: Option<String>
    )
    {
        self.http.set_tokens(access_token, refresh_token, None);
    }

    fn access_token(&self) -> Option<String> {
        if let Ok(token) = self.http.require_token() {
            Some(token)
        } else {
            None
        }
    }

    fn refresh_token(&self) -> Option<String> {
        if let Ok(token) = self.http.require_refresh() {
            Some(token)
        } else {
            None
        }
    }
}