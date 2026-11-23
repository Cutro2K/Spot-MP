use std::{collections::HashMap};

use crate::{
    domain::spot_mp_errors::SpotMpError, services::spotify_api::{
        auth::{ 
            browser_opener::SystemBrowserOpener, callback_server::AuthCallbackServer, oauth_config::OAuthConfig, pkce::SpotifyAuth}, client::http_client::SpotifyHttpClient, models::SpotifyTokenResponse, traits::{AuthManager, BrowserOpener}
    }
};


const SPOTIFY_AUTH_PATH: &str = "/authorize";
const SPOTIFY_TOKEN_PATH: &str = "/api/token";



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
        let auth_url = self.http.accounts_url(SPOTIFY_AUTH_PATH);
        let url = reqwest::Url::parse_with_params(&auth_url, auth_params)?;
        

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
        let token_url = self.http.accounts_url(SPOTIFY_TOKEN_PATH);
        let token_data = self
            .http
            .post(&token_url)
            .without_auth()
            .form(&params)
            .send_json::<SpotifyTokenResponse>()
            .await?;

        // Validating refresh_token
        let refresh_token = token_data.refresh_token.ok_or_else(|| {
            SpotMpError::Error("Cannot retrieve the refresh token value after login!".to_string())
        })?;
       
        self.http.set_session(
            Some(client_id.to_string()),
            Some(token_data.access_token),
            Some(refresh_token),
        );
        Ok(())
    }

    fn restore_session(
        &mut self,
        client_id: String,
        access_token: String,
        refresh_token: String,
    ) {
        self.http
            .set_session(Some(client_id), Some(access_token), Some(refresh_token));
    }

    fn logout(&self) {
        self.http.set_session(None, None, None);
    }

    fn clear_tokens(&self) {
        self.http.clear_web_api_tokens();
    }

    async fn renew_token(&self) -> Result<(), SpotMpError> {
        let (epoch_before, client_id, mut refresh_token) =
            self.http.snapshot_refresh_credentials()?;

        let mut params = HashMap::new();
        params.insert("grant_type", "refresh_token");
        params.insert("refresh_token", refresh_token.as_str());
        params.insert("client_id", client_id.as_str());

        let token_url = self.http.accounts_url(SPOTIFY_TOKEN_PATH);
        let token_data = self
            .http
            .post(&token_url)
            .without_auth()
            .form(&params)
            .send_json::<SpotifyTokenResponse>()
            .await?;

        if let Some(new_refresh_token) = token_data.refresh_token {
            refresh_token = new_refresh_token;
        }

        self.http
            .update_tokens_if_epoch(epoch_before, token_data.access_token, refresh_token);
        Ok(())
    }

    fn access_token(&self) -> Option<String> {
        self.http.access_token()
    }

    fn refresh_token(&self) -> Option<String> {
        self.http.refresh_token()
    }

    fn client_id(&self) -> Option<String> {
        self.http.client_id()
    }

    fn session_epoch(&self) -> u64 {
        self.http.session_epoch()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, RwLock};

    use super::*;
    use crate::services::spotify_api::client::http_client::TokenState;

    struct NoOpBrowser;
    impl BrowserOpener for NoOpBrowser {
        fn open(&self, _url: &str) -> Result<(), SpotMpError> {
            Ok(())
        }
    }

    fn new_auth_service() -> (AuthService<NoOpBrowser>, SpotifyHttpClient) {
        let http = SpotifyHttpClient::new(Arc::new(RwLock::new(TokenState::default())));
        let auth = AuthService::with_browser(http.clone(), OAuthConfig::default(), NoOpBrowser);
        (auth, http)
    }

    #[test]
    fn restore_clear_tokens_and_logout_manage_session_and_client_id() {
        let (mut auth, http) = new_auth_service();

        auth.restore_session(
            "client-abc".to_string(),
            "access-1".to_string(),
            "refresh-1".to_string(),
        );
        assert_eq!(auth.client_id().as_deref(), Some("client-abc"));
        assert_eq!(auth.access_token().as_deref(), Some("access-1"));
        assert_eq!(auth.refresh_token().as_deref(), Some("refresh-1"));
        let epoch_after_restore = http.session_epoch();

        // Web-API-only clear via shared &self preserves client_id and increments epoch.
        let shared_auth: &AuthService<NoOpBrowser> = &auth;
        shared_auth.clear_tokens();
        assert_eq!(auth.access_token(), None);
        assert_eq!(auth.refresh_token(), None);
        assert_eq!(auth.client_id().as_deref(), Some("client-abc"));
        assert!(http.session_epoch() > epoch_after_restore);

        // Full logout clears client_id and tokens and increments epoch.
        let epoch_after_clear = http.session_epoch();
        auth.logout();
        assert_eq!(auth.client_id(), None);
        assert_eq!(auth.access_token(), None);
        assert_eq!(auth.refresh_token(), None);
        assert!(http.session_epoch() > epoch_after_clear);
    }

    #[tokio::test]
    async fn renew_token_uses_shared_self_and_errors_when_credentials_missing() {
        let (mut auth, _http) = new_auth_service();

        // Missing client_id and refresh_token -> SessionExpired via &self
        let shared_auth: &AuthService<NoOpBrowser> = &auth;
        let err = shared_auth.renew_token().await.unwrap_err();
        assert!(
            matches!(err, SpotMpError::SessionExpired(_)),
            "expected SessionExpired when client_id missing, got: {err:?}"
        );

        // With client_id present but tokens cleared -> SessionExpired via &self
        auth.restore_session(
            "client-abc".to_string(),
            "access-1".to_string(),
            "refresh-1".to_string(),
        );
        auth.clear_tokens();
        let err = auth.renew_token().await.unwrap_err();
        assert!(
            matches!(err, SpotMpError::SessionExpired(_)),
            "expected SessionExpired when refresh_token missing, got: {err:?}"
        );
    }
}