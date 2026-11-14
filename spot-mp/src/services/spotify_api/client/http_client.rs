use std::sync::Arc;
use std::sync::RwLock;
use reqwest::{Client};

use crate::{domain::spot_mp_errors::{SpotMpError, Status}, services::spotify_api::client::request_builder::SpotifyRequestBuilder};

// For handling automatic refresh with time conditions
#[derive(Default)]
pub struct TokenState {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<std::time::Instant>,
}

#[derive(Clone)]
pub struct SpotifyHttpClient {
    client: Client,
    tokens: Arc<RwLock<TokenState>>,
}

impl SpotifyHttpClient {
    pub fn new(tokens: Arc<RwLock<TokenState>>) -> Self {
        Self {
            client: Client::new(),
            tokens,
        }
    }

    // Centralized error mapping function for http results.
    pub fn map_error_status(status_code: u16, code_text: String) -> SpotMpError {
        match status_code {
            400 => SpotMpError::SpotifyErrorStatus(Status::BadRequest(code_text)),
            401 => SpotMpError::SpotifyErrorStatus(Status::Unauthorized(code_text)),
            429 => SpotMpError::SpotifyErrorStatus(Status::TooManyRequest(code_text)),
            _ => SpotMpError::SpotifyErrorStatus(Status::Unknown(status_code, code_text)),
        }
    }

    // Request an access token safety
    pub fn require_token(&self) -> Result<String, SpotMpError> {
        let guard = self.tokens.read().unwrap();
        guard.access_token.clone().ok_or_else(|| {
            SpotMpError::SpotifyErrorStatus(Status::Unauthorized(
                "The access token is missing!".to_string(),
            ))
        })
    }
    pub fn require_refresh(&self) -> Result<String, SpotMpError> {
        self.tokens.read().unwrap().refresh_token.clone().ok_or_else(|| {
            SpotMpError::Error(String::from(
                "Tried to renew token before authentication (refresh_token is none)",
            ))
        })
    }

    // --- Entry points for builder pattern ---
    pub fn get(&self, url: &str) -> SpotifyRequestBuilder<'_> {
        SpotifyRequestBuilder::new(self, self.client.get(url))
    }
    pub fn post(&self, url: &str) -> SpotifyRequestBuilder<'_> {
        SpotifyRequestBuilder::new(self, self.client.post(url))
    }
    pub fn delete(&self, url: &str) -> SpotifyRequestBuilder<'_> {
        SpotifyRequestBuilder::new(self, self.client.delete(url))
    }

    pub fn set_tokens(
        &self,
        access_token: Option<String>,
        refresh_token: Option<String>,
        expires_at: Option<std::time::Instant>,
    ) {
        let mut guard = self.tokens.write().unwrap();
        guard.access_token = access_token;
        guard.refresh_token = refresh_token;
        guard.expires_at = expires_at;
    }
    pub fn access_token(&self) -> Option<String> {
        self.tokens.read().unwrap().access_token.clone()
    }
    pub fn refresh_token(&self) -> Option<String> {
        self.tokens.read().unwrap().refresh_token.clone()
    }
}