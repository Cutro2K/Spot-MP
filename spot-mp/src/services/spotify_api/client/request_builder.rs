use reqwest::{RequestBuilder, Response};
use serde::{Serialize, de::DeserializeOwned};

use crate::{domain::spot_mp_errors::SpotMpError, services::spotify_api::client::http_client::SpotifyHttpClient};

pub struct SpotifyRequestBuilder<'a> {
    http: &'a SpotifyHttpClient,
    builder: RequestBuilder,
    requires_auth: bool,
}

impl<'a> SpotifyRequestBuilder<'a> {
    pub fn new(http: &'a SpotifyHttpClient, builder: RequestBuilder) -> Self {
        Self {
            http,
            builder,
            requires_auth: true, // By default the calls has to be authenticated
        }
    }

    /// Set requires_auth in false for not authenticated API calls
    pub fn without_auth(mut self) -> Self {
        self.requires_auth = false;
        self
    }

    pub fn json<B: Serialize + ?Sized>(mut self, body: &B) -> Self {
        self.builder = self.builder.json(body);
        self
    }

    pub fn form<F: Serialize + ?Sized>(mut self, params: &F) -> Self {
        self.builder = self.builder.form(params);
        self
    }

    pub fn query<Q: Serialize + ?Sized>(mut self, query: &Q) -> Self {
        self.builder = self.builder.query(query);
        self
    }

    /// Centralized excecution with error handler.
    async fn execute(mut self) -> Result<Response, SpotMpError> {
        if self.requires_auth {
            let token = self.http.require_token().await?;
            self.builder = self.builder.bearer_auth(token);
        }

        let response = self.builder.send().await?;
        if response.status().is_success() {
            Ok(response)
        } else {
            let status_code = response.status().as_u16();
            let code_text = response.text().await?;
            Err(SpotifyHttpClient::map_error_status(status_code, code_text))
        }
    }

    /// Method to specify to the builder that the call retrieves a JSON
    pub async fn send_json<T: DeserializeOwned>(self) -> Result<T, SpotMpError> {
        let response = self.execute().await?;
        Ok(response.json::<T>().await?)
    }

    /// Method to excecute without result
    pub async fn send_empty(self) -> Result<(), SpotMpError> {
        self.execute().await?;
        Ok(())
    }
}