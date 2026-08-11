use reqwest::{Client};
use std::{collections::HashMap};
use webbrowser;
use futures::future::try_join_all;
use crate::spotify_api::models::{SpotifyTokenResponse, GetUserPlaylistResponse, GetPlaylistSongsResponse};
use crate::domain::song::{Song, SongOrigin, SpotifySongData};
use crate::domain::playlist::{Playlist, PlaylistOrigin, SpotifyPlaylistData};
use crate::spotify_api::{spotify_auth::SpotifyAuth, auth_callback_server::AuthCallbackServer};
use crate::domain::spot_mp_errors::{SpotMpError, Status};

pub struct SpotifyApiClient {
    access_token: Option<String>,
    refresh_token: Option<String>, 
    client: Client,    
}
    
impl Default for SpotifyApiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl SpotifyApiClient {
    // --- new ---
    pub fn new() -> Self {
        Self {
            access_token: None,
            refresh_token: None,
            client: reqwest::Client::new(),
        }
    }
    
    // --- Getters: ---
    pub fn access_token(&self) -> Option<String> {
        self.access_token.clone()
    }
    pub fn refresh_token(&self) -> Option<String> {
        self.refresh_token.clone()
    }

    // --- Function 1: Login in Spotify to get tokens --- 
    pub async fn login(&mut self, client_id: String) -> Result<String, SpotMpError> {
        // Common Variables:
        const REDIRECT_URI: &str = "http://127.0.0.1:8888/callback";
        const SCOPE: &str = "playlist-read-private playlist-read-collaborative playlist-modify-private playlist-modify-public streaming user-modify-playback-state user-read-playback-state user-library-read";
        
        // First Step: Getting Code Verifier & Code Challenge:
        let auth_codes = SpotifyAuth::new();
        let code_verifier = auth_codes.code_verifier();
        let code_challenge = auth_codes.code_challenge();

        // Second Step: Request user authentication: 
        let mut params_authentication = HashMap::new();
        params_authentication.insert("client_id", client_id.clone());
        params_authentication.insert("response_type", "code".to_string());
        params_authentication.insert("redirect_uri", REDIRECT_URI.to_string());
        params_authentication.insert("scope", SCOPE.to_string());
        params_authentication.insert("code_challenge_method", "S256".to_string());
        params_authentication.insert("code_challenge", code_challenge);
    
        let url = reqwest::Url::parse_with_params("https://accounts.spotify.com/authorize", &params_authentication)?;
        
        if webbrowser::open(url.as_str()).is_err() {
            println!("Error ocurred when trying to open the web browser!");
        }
        
        let code = AuthCallbackServer::listen_for_code()?; 

        // Third Step Requesting an access & refresh token:
        let url = "https://accounts.spotify.com/api/token";
        let mut params = HashMap::new();
        params.insert("client_id", client_id);
        params.insert("grant_type", "authorization_code".to_string());
        params.insert("redirect_uri", REDIRECT_URI.to_string());
        params.insert("code", code);
        params.insert("code_verifier", code_verifier);
        
        let response = self.client.post(url).form(&params).send().await?;
        
        if response.status().is_success() {
            let token_data = response.json::<SpotifyTokenResponse>().await?;

            self.access_token = Some(token_data.access_token);

            if let Some(refresh) = token_data.refresh_token {
                self.refresh_token = Some(refresh);
            }
            
            Ok(self.refresh_token.clone().expect("Can not retrieve the refresh token value after login!")) // This value is returned for saving the token in auth_storage.rs
        }
        else {
            let status_code = response.status().as_u16();
            let code_text = response.text().await?;

            match status_code {
                400 => Err(SpotMpError::SpotifyErrorStatus(Status::BadRequest(code_text))),
                401 => Err(SpotMpError::SpotifyErrorStatus(Status::Unauthorized(code_text))),
                429 => Err(SpotMpError::SpotifyErrorStatus(Status::TooManyRequest(code_text))),
                _ => Err(SpotMpError::SpotifyErrorStatus(Status::Unknown(status_code, code_text))),
            }
        }
    }

    // --- Function 2: Renew access token --- 
    pub async fn renew_token(&mut self, client_id: String) -> Result<String, SpotMpError> {
        // Building the request:
        let url = "https://accounts.spotify.com/api/token";
        let refresh_token = self.refresh_token().ok_or(SpotMpError::Error("Tried to renew token before authentication (refresh_token is none)".to_string()))?;

        // Parameters:
        let mut params = HashMap::new();
        params.insert("grant_type", "refresh_token");
        params.insert("refresh_token", refresh_token.as_str());
        params.insert("client_id", client_id.as_str());
        
        // Making the request
        let response = self.client.post(url).header("Content-Type", "application/x-www-form-urlencoded").form(&params).send().await?;
        
        // Handling posible errors
        if response.status().is_success() {
            let token_data = response.json::<SpotifyTokenResponse>().await?;

            self.access_token = Some(token_data.access_token);
            
            if let Some(new_refresh) = token_data.refresh_token {
                self.refresh_token = Some(new_refresh);
            }

            Ok(self.refresh_token.clone().expect("Can not retrieve the refresh token value after login!")) // This value is returned for saving the token in auth_storage.rs
        }
        else {
            let status_code = response.status().as_u16();
            let code_text = response.text().await?;
            match status_code {
                400 => Err(SpotMpError::SpotifyErrorStatus(Status::BadRequest(code_text))),
                401 => Err(SpotMpError::SpotifyErrorStatus(Status::Unauthorized(code_text))),
                429 => Err(SpotMpError::SpotifyErrorStatus(Status::TooManyRequest(code_text))),
                _ => Err(SpotMpError::SpotifyErrorStatus(Status::Unknown(status_code, code_text))),
            }
        }
    }
    
    // --- Function 3: Import Playlists ---
    pub async fn import_playlist(&self, client_id: &str) -> Result<Vec<Playlist>, SpotMpError> {
        let mut result: Vec<Playlist> = Vec::new();
    
        let mut next_url = Some("https://api.spotify.com/v1/me/playlists?limit=50".to_string());    
    
        while let Some(url) = next_url {
            let response = self.client.get(&url).bearer_auth(self.access_token.as_ref().unwrap()).send().await?;
            let status = response.status();
            
            if status.is_success() {
                let get_playlist_response = response.json::<GetUserPlaylistResponse>().await?;
                let mut future_peticions = Vec::new();

                for item in get_playlist_response.items {
                    let is_modifiable = item.collaborative || item.owner.id == client_id;
                    let playlist_name = item.name.clone();
                    let playlist_id = item.id.clone();

                    future_peticions.push(async move {
                        let songs = self.get_playlist_songs(&playlist_id).await?;

                        Ok::<Playlist, SpotMpError>(Playlist::new(
                                songs,
                                is_modifiable,
                                playlist_name,
                                PlaylistOrigin::Spotify(SpotifyPlaylistData { id_playlist: playlist_id }),
                        ))
                    });
                }
                
                let processed_playlist = try_join_all(future_peticions).await?;

                result.extend(processed_playlist);
                next_url = get_playlist_response.next;
            } else {
                let status_code = status.as_u16();
                let code_text = response.text().await?; 

                return match status_code {
                    400 => Err(SpotMpError::SpotifyErrorStatus(Status::BadRequest(code_text))),
                    401 => Err(SpotMpError::SpotifyErrorStatus(Status::Unauthorized(code_text))),
                    429 => Err(SpotMpError::SpotifyErrorStatus(Status::TooManyRequest(code_text))),
                    _ => Err(SpotMpError::SpotifyErrorStatus(Status::Unknown(status_code, code_text))),
                };
            }
        }

        Ok(result)
    }

    // -- Auxiliar Function: Get playlist songs --
    async fn get_playlist_songs(&self, playlist_id: &str) -> Result<Vec<Song>, SpotMpError> {
        let mut result: Vec<Song> = Vec::new();
        let mut next_url = Some(format!("https://api.spotify.com/v1/playlists/{}/items?limit=50", playlist_id));
        
        while let Some(url) = next_url {
            let response = self.client.get(&url).bearer_auth(self.access_token.as_ref().unwrap()).send().await?;
            let status = response.status();
            
            if status.is_success() {
                let song_response = response.json::<GetPlaylistSongsResponse>().await?;
                
                for item in song_response.items {
                    result.push(Song::new(
                            None, 
                            item.item.name, 
                            (item.item.duration_ms/1000).into(), 
                            item.item.album.images.first().map(|img| img.url.clone()).unwrap_or_else(|| String::from("ruta/a/tu/broken-image.png")), 
                            SongOrigin::Spotify(SpotifySongData {spotify_id: item.item.id})
                            ));                   
                }
                next_url = song_response.next;
            } else {
                let status_code = status.as_u16();
                let code_text = response.text().await?; 

                return match status_code {
                    400 => Err(SpotMpError::SpotifyErrorStatus(Status::BadRequest(code_text))),
                    401 => Err(SpotMpError::SpotifyErrorStatus(Status::Unauthorized(code_text))),
                    429 => Err(SpotMpError::SpotifyErrorStatus(Status::TooManyRequest(code_text))),
                    _ => Err(SpotMpError::SpotifyErrorStatus(Status::Unknown(status_code, code_text))),
                };
            }
        }
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_login_real_interactive() {
        // 1.Instantiate the client using the method you defined
        let mut client = SpotifyApiClient::new();
        
        // REPLACE THIS STRING WITH YOUR REAL SPOTIFY CLIENT ID
        let my_real_client_id = "850a0abcc2474ff5aad399aeb9489d06".to_string();

        println!("WARNING: The test will open the browser. Please authorize the app to continue.");

        // 2. Launch the login process. 
        // The test will pause on this line waiting for your interaction in the browser.
        let result = client.login(my_real_client_id.clone()).await;

        // 3. Main Assert: Ensure the result was Ok(())
        assert!(result.is_ok(), "The login process failed: {:?}", result.err());

        // 4. State Assert: Check that the tokens were actually saved in the instance
        let obtained_token = client.access_token();
        assert!(obtained_token.is_some(), "Login was Ok, but the access_token is None");

        let playlists_result = client.import_playlist(&my_real_client_id).await;
        assert!(playlists_result.is_ok(), "The import_playlist function failed: {:?}", playlists_result.err());
        
        let playlists = playlists_result.unwrap();
        
        for playlist in &playlists {
            println!("{}", playlist.playlist_name());
        }
        println!("Test passed! Successfully imported {} playlists.", playlists.len());
        assert!(!playlists.is_empty(), "0 playlists were returned.");
    }
}
