use log::error;
use reqwest::{Client};
use std::{collections::HashMap};
use webbrowser;
use futures::future::try_join_all;
use futures::try_join;
use serde_json::json;
use crate::services::spotify_api::models::{GetAlbumSongsResponse, GetPlaylistSongsResponse, GetUserAlbumsRespose, GetUserPlaylistsResponse, SearchSongResponse, SpotifyTokenResponse, PlaylistItem};
use crate::domain::song::{Song, SongOrigin, SpotifySongData};
use crate::domain::playlist::{Playlist, PlaylistOrigin, SpotifyPlaylistData};
use crate::services::spotify_api::{spotify_auth::SpotifyAuth, auth_callback_server::AuthCallbackServer};
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
    // --- CONSTRUCTOR ---
    pub fn new() -> Self {
        Self {
            access_token: None,
            refresh_token: None,
            client: reqwest::Client::new(),
        }
    }
    
    // --- GETTERS ---
    pub fn access_token(&self) -> Option<&str> {
        match &self.access_token {
            Some(token) => Some(token),
            None => None,
        }
    }
    pub fn refresh_token(&self) -> Option<&str> {
        match &self.refresh_token {
        Some(token) => Some(token),
            None => None,
        }
    }
    
    // --- AUXILIAR FUNCIONS ---
    
    // -- Get playlist tracks --
    async fn get_playlist_tracks(&self, playlist_id: &str) -> Result<Vec<Song>, SpotMpError> {
        let mut result: Vec<Song> = Vec::new();
    let mut next_url = Some(format!("https://api.spotify.com/v1/playlists/{}/items?limit=50", playlist_id));
        let token = self.access_token.as_ref().expect("The access token is missing!");

        while let Some(url) = next_url {
            
            let response = self.client.get(&url).bearer_auth(token).send().await?;
            let status = response.status();
            
            if status.is_success() {
                let song_response = response.json::<GetPlaylistSongsResponse>().await?;
                
                for item in song_response.items {
                    result.push(Song::new(
                            None, 
                            item.item.name, 
                            (item.item.duration_ms/1000).into(), 
                             item.item.album.images.first().map(|img| img.url.clone()).unwrap_or_else(|| String::from("/home/cutro2k/Documents/Projects/Spot-MP/spot-mp/assets/images/broken-image.png")), // I have to change this
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
    
    // -- Get album traks ---
    async fn get_album_tracks(&self, album_id: &str, img_path: &str) -> Result<Vec<Song>, SpotMpError> {
        let mut result: Vec<Song> = Vec::new();
        let mut next_url = Some(format!("https://api.spotify.com/v1/albums/{}/tracks?limit=50", album_id));
        let token = self.access_token.as_ref().expect("The access token is missing!");

        while let Some(url) = next_url {
            let response = self.client.get(&url).bearer_auth(token).send().await?;
            let status = response.status();

            if status.is_success() {
                let song_response = response.json::<GetAlbumSongsResponse>().await?;

                for item in song_response.items {
                    result.push(Song::new(
                            None, 
                            item.name,
                            (item.duration_ms/1000).into(),
                            img_path.to_owned(),
                            SongOrigin::Spotify(SpotifySongData { spotify_id: item.id }),
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
    
    // -- Import Liked Playlist --
    async fn import_liked_playlist(&self, client_id: &str) -> Result<Vec<Playlist>, SpotMpError> {
        let mut result: Vec<Playlist> = Vec::new();
        let mut next_url = Some("https://api.spotify.com/v1/me/playlists?limit=50".to_string());    
        let token = self.access_token.as_ref().expect("The access token is missing!");

        while let Some(url) = next_url {
            let response = self.client.get(&url).bearer_auth(token).send().await?;
            let status = response.status();
            
            if status.is_success() {
                let get_playlist_response = response.json::<GetUserPlaylistsResponse>().await?;

                let mut future_peticions = Vec::new();

                for item in get_playlist_response.items {
                    let name = item.name.clone();
                    let is_modifiable = item.collaborative || item.owner.id == client_id;
                    let playlist_id = item.id;

                    future_peticions.push( async move {
                let songs = self.get_playlist_tracks(&playlist_id).await?;

                        Ok::<Playlist, SpotMpError>(Playlist::new(
                                songs, 
                                is_modifiable, 
                                name, 
                                PlaylistOrigin::Spotify(SpotifyPlaylistData { id_playlist: playlist_id })
                        ))
                    });
                }
                
                // Start the concurrency execution 
                let proccesed_playlist = try_join_all(future_peticions).await?;
                result.extend(proccesed_playlist);

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
    // -- Import Liked Albums --
    async fn import_liked_albums(&self) -> Result<Vec<Playlist>, SpotMpError> {
        let mut result: Vec<Playlist> = Vec::new();
        let mut next_url = Some("https://api.spotify.com/v1/me/albums?limit=50".to_string());    
        let token = self.access_token.as_ref().expect("The access token is missing!");

        while let Some(url) = next_url {
            let response = self.client.get(&url).bearer_auth(token).send().await?;
            let status = response.status();

            if status.is_success() {
                let get_album_response = response.json::<GetUserAlbumsRespose>().await?;                
                let mut future_peticions = Vec::new();

                for item in get_album_response.items {
                    let name = item.album.name.clone();
                    let album_id = item.album.id;
                    let img_path = item.album.images.first().map(|img| img.url.clone()).unwrap_or_else(|| String::from("/home/cutro2k/Documents/Projects/Spot-MP/spot-mp/assets/images/broken-image.png"));

                    future_peticions.push( async move {
                        let songs = self.get_album_tracks(&album_id, &img_path).await?;

                        Ok::<Playlist, SpotMpError>(Playlist::new( 
                                songs, 
                                false, 
                                name, 
                                PlaylistOrigin::Spotify(SpotifyPlaylistData { id_playlist: album_id })
                        ))
                    });
                }

                let processed_playlist = try_join_all(future_peticions).await?;
                result.extend(processed_playlist);

                next_url = get_album_response.next;
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

    // --- MAIN FUNCTIONALITY ---

    // --- Function 1: Login in Spotify to get tokens --- 
    pub async fn login(&mut self, client_id: &str) -> Result<String, SpotMpError> {
        // Common Variables:
        const REDIRECT_URI: &str = "http://127.0.0.1:8888/callback";
        const IMAGE_SCOPE: &str = "ugc-image-upload";
        const PLAYLIST_SCOPE: &str = "playlist-read-private playlist-read-collaborative playlist-modify-private playlist-modify-public";
        const USER_SCOPE: &str = "user-modify-playback-state user-read-playback-state user-library-read user-read-currently-playing user-read-email user-read-private";
        const OTHER_SCOPE: &str = "streaming app-remote-control";
        
        let scopes = PLAYLIST_SCOPE.to_owned() + " " + USER_SCOPE + " " + OTHER_SCOPE + " " + IMAGE_SCOPE; 
        
        // First Step: Getting Code Verifier & Code Challenge:
        let auth_codes = SpotifyAuth::new();
        let code_verifier = auth_codes.code_verifier();
        let code_challenge = auth_codes.code_challenge();

        // Second Step: Request user authentication: 
        let mut params_authentication = HashMap::new();
        params_authentication.insert("client_id", client_id);
        params_authentication.insert("response_type", "code");
        params_authentication.insert("redirect_uri", REDIRECT_URI);
        params_authentication.insert("scope", &scopes);
        params_authentication.insert("code_challenge_method", "S256");
        params_authentication.insert("code_challenge", code_challenge);
    
        let url = reqwest::Url::parse_with_params("https://accounts.spotify.com/authorize", &params_authentication)?;
        
        if let Err(e) = webbrowser::open(url.as_str()) {
            error!("Error ocurred when trying to open the web browser: {}", e);
        }
        
        let code = AuthCallbackServer::listen_for_code()?; 

        // Third Step Requesting an access & refresh token:
        let url = "https://accounts.spotify.com/api/token";
        let mut params = HashMap::new();
        params.insert("client_id", client_id);
        params.insert("grant_type", "authorization_code");
        params.insert("redirect_uri", REDIRECT_URI);
        params.insert("code", &code);
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
    pub fn logout(&mut self) {
        self.access_token = None;
        self.refresh_token = None;
    }

    // --- Function 2: Renew access token --- 
    pub async fn renew_token(&mut self, client_id: &str) -> Result<String, SpotMpError> {
        // Building the request:
        let url = "https://accounts.spotify.com/api/token";
        let refresh_token = self.refresh_token().ok_or(SpotMpError::Error("Tried to renew token before authentication (refresh_token is none)".to_string()))?;

        // Parameters:
        let mut params = HashMap::new();
        params.insert("grant_type", "refresh_token");
        params.insert("refresh_token", refresh_token);
        params.insert("client_id", client_id);
        
        // Making the request
        let response = self.client.post(url).header("Content-Type", "application/x-www-form-urlencoded").form(&params).send().await?;
        
        // Handling posible errors
        if response.status().is_success() {
            let token_data = response.json::<SpotifyTokenResponse>().await?;

            self.access_token = Some(token_data.access_token);
            
            if let Some(new_refresh) = token_data.refresh_token {
                self.refresh_token = Some(new_refresh);
            }

            Ok(self.refresh_token.clone().expect("Can not retrieve the refresh token value after refresh!")) // This value is returned for saving the token in auth_storage.rs
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
    
    // --- Function 3: Import Albums and Playlists ---
    pub async fn import_liked_albums_and_playlist(&self, client_id: &str) -> Result<Vec<Playlist>, SpotMpError> {
        let mut result: Vec<Playlist> = Vec::new();
        
        let playlists_fut = self.import_liked_playlist(client_id);
        let albums_fut = self.import_liked_albums();
        let (playlists, albums) = try_join!(playlists_fut, albums_fut)?;

        result.extend(playlists);
        result.extend(albums);
        Ok(result)
    }
    
    // --- Function 4: Search song whit a specific query ---
    pub async fn search_song(&self, query: &str) -> Result<Vec<Song>, SpotMpError> {
        let mut result = Vec::new();
        let url = format!("https://api.spotify.com/v1/search?q={}&type=track&limit=6&offset=6", query);
        let token = self.access_token.as_ref().expect("The access token is missing!");
       
        let response = self.client.get(&url).bearer_auth(token).send().await?;

        if response.status().is_success() {
            let search_response = response.json::<SearchSongResponse>().await?;
            
            for item in search_response.tracks.items {
                result.push(Song::new(
                    None,
                    item.name.clone(),
                    (item.duration_ms/1000).into(),
                    item.album.images.first().map(|img| img.url.clone()).unwrap_or_else(|| String::from("/home/cutro2k/Documents/Projects/Spot-MP/spot-mp/assets/images/broken-image.png")), // I have to change this
                    SongOrigin::Spotify(SpotifySongData { spotify_id: item.id }),
                ));
            }
        }
        else {
            let status_code = response.status().as_u16();
            let code_text = response.text().await?;
            return match status_code {
                400 => Err(SpotMpError::SpotifyErrorStatus(Status::BadRequest(code_text))),
                401 => Err(SpotMpError::SpotifyErrorStatus(Status::Unauthorized(code_text))),
                429 => Err(SpotMpError::SpotifyErrorStatus(Status::TooManyRequest(code_text))),
                _ => Err(SpotMpError::SpotifyErrorStatus(Status::Unknown(status_code, code_text))),
            }
        }
        Ok(result)
    }

    // --- Function 5: Add song to an existing playlist --- 
    pub async fn add_song_to_playlist(&self, playlist_id: &str, song_id: &str) -> Result<(), SpotMpError> {
        let url =  format!("https://api.spotify.com/v1/playlists/{}/items", playlist_id);
        let token = self.access_token.as_ref().expect("The access token is missing!");
       
        let body = json!({
            "uris": [format!("spotify:track:{}", song_id)],
        });

        let response = self.client.post(&url).json(&body).bearer_auth(token).send().await?;

        let status_code = response.status().as_u16();
        let code_text = response.text().await?;
     
        match status_code {
            201 => Ok(()),
            400 => Err(SpotMpError::SpotifyErrorStatus(Status::BadRequest(code_text))),
            401 => Err(SpotMpError::SpotifyErrorStatus(Status::Unauthorized(code_text))),
            429 => Err(SpotMpError::SpotifyErrorStatus(Status::TooManyRequest(code_text))),
            _ => Err(SpotMpError::SpotifyErrorStatus(Status::Unknown(status_code, code_text))),
        }
    }

    // --- Function 6: remove song to an existing playlist --- 
    pub async fn remove_song_to_playlist(&self, playlist_id: &str, song_id: &str) -> Result<(), SpotMpError>{
        let url = format!("https://api.spotify.com/v1/playlists/{}/items", playlist_id);
        let token = self.access_token.as_ref().expect("The access token is missing!");
        
        let body = json!({
            "items": [
                {
                    "uri": format!("spotify:track:{}", song_id),
                }
            ]
        });
        
        let response = self.client.delete(&url).json(&body).bearer_auth(token).send().await?;
        
        let status_code = response.status().as_u16();
        let code_text = response.text().await?;

        match status_code {
            200 => Ok(()),
            400 => Err(SpotMpError::SpotifyErrorStatus(Status::BadRequest(code_text))),
            401 => Err(SpotMpError::SpotifyErrorStatus(Status::Unauthorized(code_text))),
            429 => Err(SpotMpError::SpotifyErrorStatus(Status::TooManyRequest(code_text))),
            _ => Err(SpotMpError::SpotifyErrorStatus(Status::Unknown(status_code, code_text))),
        }
    }

    // --- Function 7: create playlist whit an optional song ---
    pub async fn create_playlist(&self, playlist_name: &str, public: bool, song_to_add: Option<&str>) -> Result<Playlist, SpotMpError> {
        let url = "https://api.spotify.com/v1/me/playlists";
        let token = self.access_token.as_ref().expect("The access token is missing!");
    
        let body = json!({
            "name": playlist_name,
            "public": public,
        });
        
        let response = self.client.post(url).json(&body).bearer_auth(token).send().await?;
        
        if response.status().is_success() {
            let playlist_response = response.json::<PlaylistItem>().await?;
            
            let new_playlist = Playlist::new(
                Vec::new(),
                true, 
                playlist_response.name, 
                PlaylistOrigin::Spotify(SpotifyPlaylistData { id_playlist: playlist_response.id })
            );
            
            if let Some(song_id) = song_to_add {
                self.add_song_to_playlist(&new_playlist.id_playlist(), song_id).await?;
            }
            
            Ok(new_playlist)
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
}
