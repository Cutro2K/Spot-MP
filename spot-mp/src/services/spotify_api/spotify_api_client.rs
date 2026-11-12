use crate::domain::playlist::{Playlist, PlaylistOrigin, SpotifyPlaylistData};
pub use crate::domain::playlist::SpotifySearchResponse;
use crate::domain::song::{Song, SongOrigin, SpotifySongData};
use crate::domain::spot_mp_errors::{SpotMpError, Status};
use crate::services::spotify_api::models::{
    ArtistItem, GetAlbumSongsResponse, GetPlaylistSongsResponse, GetUserAlbumsRespose,
    GetUserPlaylistsResponse, PlaylistItem, SearchQueryResponse, SpotifyTokenResponse,
    UserProfileResponse,
};
use crate::services::spotify_api::{
    auth_callback_server::AuthCallbackServer, spotify_auth::SpotifyAuth,
};
use futures::future::{join, try_join_all};
use futures::try_join;
use log::error;
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use webbrowser;

pub struct SpotifyApiClient {
    access_token: Option<String>,
    refresh_token: Option<String>,
    user_id: Option<String>,
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
            user_id: None,
            client: reqwest::Client::new(),
        }
    }

    // --- GETTERS & SESSION RESTORATION ---
    pub fn access_token(&self) -> Option<&str> {
        self.access_token.as_deref()
    }

    pub fn refresh_token(&self) -> Option<&str> {
        self.refresh_token.as_deref()
    }

    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    pub fn set_tokens(&mut self, access_token: Option<String>, refresh_token: Option<String>) {
        self.access_token = access_token;
        self.refresh_token = refresh_token;
    }

    fn require_token(&self) -> Result<&str, SpotMpError> {
        self.access_token.as_deref().ok_or_else(|| {
            SpotMpError::SpotifyErrorStatus(Status::Unauthorized(
                "The access token is missing!".to_string(),
            ))
        })
    }

    fn map_error_status(status_code: u16, code_text: String) -> SpotMpError {
        match status_code {
            400 => SpotMpError::SpotifyErrorStatus(Status::BadRequest(code_text)),
            401 => SpotMpError::SpotifyErrorStatus(Status::Unauthorized(code_text)),
            429 => SpotMpError::SpotifyErrorStatus(Status::TooManyRequest(code_text)),
            _ => SpotMpError::SpotifyErrorStatus(Status::Unknown(status_code, code_text)),
        }
    }

    fn format_artists(artists: &[ArtistItem]) -> String {
        if artists.is_empty() {
            String::from("Unknown Artist")
        } else {
            artists
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        }
    }

    fn parse_release_year(release_date: Option<&str>) -> Option<i32> {
        release_date
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse::<i32>().ok())
    }

    pub async fn fetch_current_user_id(&mut self) -> Result<String, SpotMpError> {
        if let Some(uid) = &self.user_id {
            return Ok(uid.clone());
        }
        let token = self.require_token()?;
        let response = self
            .client
            .get("https://api.spotify.com/v1/me")
            .bearer_auth(token)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            let profile = response.json::<UserProfileResponse>().await?;
            self.user_id = Some(profile.id.clone());
            Ok(profile.id)
        } else {
            let status_code = status.as_u16();
            let code_text = response.text().await?;
            Err(Self::map_error_status(status_code, code_text))
        }
    }

    // --- AUXILIARY FUNCTIONS ---

    // -- Get playlist tracks --
    async fn get_playlist_tracks(&self, playlist_id: &str) -> Result<Vec<Song>, SpotMpError> {
        let mut result: Vec<Song> = Vec::new();
        let mut next_url = Some(format!(
            "https://api.spotify.com/v1/playlists/{}/tracks?limit=50",
            playlist_id
        ));
        let token = self.require_token()?;

        while let Some(url) = next_url {
            let response = self.client.get(&url).bearer_auth(token).send().await?;
            let status = response.status();

            if status.is_success() {
                let song_response = response.json::<GetPlaylistSongsResponse>().await?;

                for item in song_response.items {
                    if let Some(track) = item.track
                        && let Some(track_id) = track.id
                    {
                        let artist = Self::format_artists(&track.artists);
                        let album_name = track.album.name.clone();
                        let release_year =
                            Self::parse_release_year(track.album.release_date.as_deref());
                        let img_url = track
                            .album
                            .images
                            .first()
                            .map(|img| img.url.clone())
                            .unwrap_or_default();

                        result.push(Song::with_metadata(
                            None,
                            track.name,
                            artist,
                            album_name,
                            release_year,
                            (track.duration_ms / 1000).into(),
                            img_url,
                            SongOrigin::Spotify(SpotifySongData {
                                spotify_id: track_id,
                            }),
                        ));
                    }
                }
                next_url = song_response.next;
            } else {
                let status_code = status.as_u16();
                let code_text = response.text().await?;
                return Err(Self::map_error_status(status_code, code_text));
            }
        }

        Ok(result)
    }

    // -- Get album tracks --
    async fn get_album_tracks(
        &self,
        album_id: &str,
        album_name: &str,
        release_year: Option<i32>,
        img_path: &str,
    ) -> Result<Vec<Song>, SpotMpError> {
        let mut result: Vec<Song> = Vec::new();
        let mut next_url = Some(format!(
            "https://api.spotify.com/v1/albums/{}/tracks?limit=50",
            album_id
        ));
        let token = self.require_token()?;

        while let Some(url) = next_url {
            let response = self.client.get(&url).bearer_auth(token).send().await?;
            let status = response.status();

            if status.is_success() {
                let song_response = response.json::<GetAlbumSongsResponse>().await?;

                for item in song_response.items {
                    let artist = Self::format_artists(&item.artists);
                    result.push(Song::with_metadata(
                        None,
                        item.name,
                        artist,
                        album_name.to_owned(),
                        release_year,
                        (item.duration_ms / 1000).into(),
                        img_path.to_owned(),
                        SongOrigin::Spotify(SpotifySongData {
                            spotify_id: item.id,
                        }),
                    ));
                }
                next_url = song_response.next;
            } else {
                let status_code = status.as_u16();
                let code_text = response.text().await?;
                return Err(Self::map_error_status(status_code, code_text));
            }
        }
        Ok(result)
    }

    // -- Import Liked Playlists --
    async fn import_liked_playlist(&self, user_id: &str) -> Result<Vec<Playlist>, SpotMpError> {
        let mut result: Vec<Playlist> = Vec::new();
        let mut next_url = Some("https://api.spotify.com/v1/me/playlists?limit=50".to_string());
        let token = self.require_token()?;

        while let Some(url) = next_url {
            let response = self.client.get(&url).bearer_auth(token).send().await?;
            let status = response.status();

            if status.is_success() {
                let get_playlist_response = response.json::<GetUserPlaylistsResponse>().await?;
                let mut future_petitions = Vec::new();

                for item in get_playlist_response.items {
                    let name = item.name.clone();
                    let is_modifiable = item.collaborative || item.owner.id == user_id;
                    let playlist_id = item.id;

                    future_petitions.push(async move {
                        let songs = self.get_playlist_tracks(&playlist_id).await?;

                        Ok::<Playlist, SpotMpError>(Playlist::new(
                            songs,
                            is_modifiable,
                            name,
                            PlaylistOrigin::Spotify(SpotifyPlaylistData {
                                id_playlist: playlist_id,
                            }),
                        ))
                    });
                }

                let processed_playlist = try_join_all(future_petitions).await?;
                result.extend(processed_playlist);

                next_url = get_playlist_response.next;
            } else {
                let status_code = status.as_u16();
                let code_text = response.text().await?;
                return Err(Self::map_error_status(status_code, code_text));
            }
        }

        Ok(result)
    }

    // -- Import Liked Albums --
    async fn import_liked_albums(&self) -> Result<Vec<Playlist>, SpotMpError> {
        let mut result: Vec<Playlist> = Vec::new();
        let mut next_url = Some("https://api.spotify.com/v1/me/albums?limit=50".to_string());
        let token = self.require_token()?;

        while let Some(url) = next_url {
            let response = self.client.get(&url).bearer_auth(token).send().await?;
            let status = response.status();

            if status.is_success() {
                let get_album_response = response.json::<GetUserAlbumsRespose>().await?;
                let mut future_petitions = Vec::new();

                for item in get_album_response.items {
                    let name = item.album.name.clone();
                    let album_id = item.album.id;
                    let release_year =
                        Self::parse_release_year(item.album.release_date.as_deref());
                    let img_path = item
                        .album
                        .images
                        .first()
                        .map(|img| img.url.clone())
                        .unwrap_or_default();

                    future_petitions.push(async move {
                        let songs = self
                            .get_album_tracks(&album_id, &name, release_year, &img_path)
                            .await?;

                        Ok::<Playlist, SpotMpError>(Playlist::new(
                            songs,
                            false,
                            name,
                            PlaylistOrigin::Spotify(SpotifyPlaylistData {
                                id_playlist: album_id,
                            }),
                        ))
                    });
                }

                let processed_playlist = try_join_all(future_petitions).await?;
                result.extend(processed_playlist);

                next_url = get_album_response.next;
            } else {
                let status_code = status.as_u16();
                let code_text = response.text().await?;
                return Err(Self::map_error_status(status_code, code_text));
            }
        }

        Ok(result)
    }

    // --- MAIN FUNCTIONALITY ---

    // --- Function 1: Login in Spotify to get tokens ---
    pub async fn login(&mut self, client_id: &str) -> Result<String, SpotMpError> {
        const REDIRECT_URI: &str = "http://127.0.0.1:8888/callback";
        const IMAGE_SCOPE: &str = "ugc-image-upload";
        const PLAYLIST_SCOPE: &str = "playlist-read-private playlist-read-collaborative playlist-modify-private playlist-modify-public";
        const USER_SCOPE: &str = "user-modify-playback-state user-read-playback-state user-library-read user-read-currently-playing user-read-email user-read-private";
        const OTHER_SCOPE: &str = "streaming app-remote-control";

        let scopes =
            format!("{PLAYLIST_SCOPE} {USER_SCOPE} {OTHER_SCOPE} {IMAGE_SCOPE}");

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

        let url = reqwest::Url::parse_with_params(
            "https://accounts.spotify.com/authorize",
            &params_authentication,
        )?;

        if let Err(e) = webbrowser::open(url.as_str()) {
            error!("Error occurred when trying to open the web browser: {}", e);
        }

        let code = AuthCallbackServer::listen_for_code().await?;

        // Third Step: Requesting an access & refresh token:
        let token_url = "https://accounts.spotify.com/api/token";
        let mut params = HashMap::new();
        params.insert("client_id", client_id);
        params.insert("grant_type", "authorization_code");
        params.insert("redirect_uri", REDIRECT_URI);
        params.insert("code", &code);
        params.insert("code_verifier", code_verifier);

        let response = self.client.post(token_url).form(&params).send().await?;

        if response.status().is_success() {
            let token_data = response.json::<SpotifyTokenResponse>().await?;

            self.access_token = Some(token_data.access_token);

            if let Some(refresh) = token_data.refresh_token {
                self.refresh_token = Some(refresh);
            }

            // Cache the authenticated user's Spotify profile ID
            let _ = self.fetch_current_user_id().await;

            self.refresh_token.clone().ok_or_else(|| {
                SpotMpError::Error(
                    "Cannot retrieve the refresh token value after login!".to_string(),
                )
            })
        } else {
            let status_code = response.status().as_u16();
            let code_text = response.text().await?;
            Err(Self::map_error_status(status_code, code_text))
        }
    }

    pub fn logout(&mut self) {
        self.access_token = None;
        self.refresh_token = None;
        self.user_id = None;
    }

    // --- Function 2: Renew access token ---
    pub async fn renew_token(&mut self, client_id: &str) -> Result<String, SpotMpError> {
        let url = "https://accounts.spotify.com/api/token";
        let refresh_token = self
            .refresh_token()
            .ok_or_else(|| {
                SpotMpError::Error(
                    "Tried to renew token before authentication (refresh_token is none)"
                        .to_string(),
                )
            })?
            .to_string();

        let mut params = HashMap::new();
        params.insert("grant_type", "refresh_token");
        params.insert("refresh_token", &refresh_token);
        params.insert("client_id", client_id);

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;

        if response.status().is_success() {
            let token_data = response.json::<SpotifyTokenResponse>().await?;

            self.access_token = Some(token_data.access_token);

            if let Some(new_refresh) = token_data.refresh_token {
                self.refresh_token = Some(new_refresh);
            }

            self.refresh_token.clone().ok_or_else(|| {
                SpotMpError::Error(
                    "Cannot retrieve the refresh token value after refresh!".to_string(),
                )
            })
        } else {
            let status_code = response.status().as_u16();
            let code_text = response.text().await?;
            Err(Self::map_error_status(status_code, code_text))
        }
    }

    // --- Function 3: Import Albums and Playlists ---
    pub async fn import_liked_albums_and_playlist(
        &mut self,
    ) -> Result<Vec<Playlist>, SpotMpError> {
        let user_id = self.fetch_current_user_id().await?;
        let mut result: Vec<Playlist> = Vec::new();

        let playlists_fut = self.import_liked_playlist(&user_id);
        let albums_fut = self.import_liked_albums();
        let (playlists, albums) = try_join!(playlists_fut, albums_fut)?;

        result.extend(playlists);
        result.extend(albums);
        Ok(result)
    }

    // --- Function 4: Search song, album, and playlist with a specific query ---
    pub async fn search_query(
        &self,
        query: &str,
    ) -> Result<Vec<SpotifySearchResponse>, SpotMpError> {
        let mut result: Vec<SpotifySearchResponse> = Vec::new();
        let url = reqwest::Url::parse_with_params(
            "https://api.spotify.com/v1/search",
            &[
                ("q", query),
                ("type", "track,album,playlist"),
                ("limit", "3"),
                ("offset", "0"),
            ],
        )?;
        let token = self.require_token()?;

        let response = self.client.get(url).bearer_auth(token).send().await?;

        if response.status().is_success() {
            let search_response = response.json::<SearchQueryResponse>().await?;

            for item in search_response.tracks.items {
                let artist = Self::format_artists(&item.artists);
                let album_name = item.album.name.clone();
                let release_year =
                    Self::parse_release_year(item.album.release_date.as_deref());
                let img_url = item
                    .album
                    .images
                    .first()
                    .map(|img| img.url.clone())
                    .unwrap_or_default();

                result.push(SpotifySearchResponse::Song(Song::with_metadata(
                    None,
                    item.name,
                    artist,
                    album_name,
                    release_year,
                    (item.duration_ms / 1000).into(),
                    img_url,
                    SongOrigin::Spotify(SpotifySongData {
                        spotify_id: item.id,
                    }),
                )));
            }

            let mut future_petitions_album = Vec::new();
            let mut future_petitions_playlist = Vec::new();

            for item in search_response.albums.items {
                let name = item.name.clone();
                let album_id = item.id;
                let release_year = Self::parse_release_year(item.release_date.as_deref());
                let img_path = item
                    .images
                    .first()
                    .map(|img| img.url.clone())
                    .unwrap_or_default();

                future_petitions_album.push(async move {
                    let songs = self
                        .get_album_tracks(&album_id, &name, release_year, &img_path)
                        .await?;

                    Ok::<Playlist, SpotMpError>(Playlist::new(
                        songs,
                        false,
                        name,
                        PlaylistOrigin::Spotify(SpotifyPlaylistData {
                            id_playlist: album_id,
                        }),
                    ))
                });
            }

            for item in search_response.playlists.items.into_iter().flatten() {
                let name = item.name.clone();
                let playlist_id = item.id;

                future_petitions_playlist.push(async move {
                    let songs = self.get_playlist_tracks(&playlist_id).await?;

                    Ok::<Playlist, SpotMpError>(Playlist::new(
                        songs,
                        false,
                        name,
                        PlaylistOrigin::Spotify(SpotifyPlaylistData {
                            id_playlist: playlist_id,
                        }),
                    ))
                });
            }

            let albums_tasks = try_join_all(future_petitions_album);
            let playlists_tasks = try_join_all(future_petitions_playlist);

            let (albums_results, playlists_results) = join(albums_tasks, playlists_tasks).await;

            for album in albums_results? {
                result.push(SpotifySearchResponse::Album(album));
            }
            for playlist in playlists_results? {
                result.push(SpotifySearchResponse::Playlist(playlist));
            }
        } else {
            let status_code = response.status().as_u16();
            let code_text = response.text().await?;
            return Err(Self::map_error_status(status_code, code_text));
        }
        Ok(result)
    }

    // --- Function 5: Add song to an existing playlist ---
    pub async fn add_song_to_playlist(
        &self,
        playlist_id: &str,
        song_id: &str,
    ) -> Result<(), SpotMpError> {
        let url = format!("https://api.spotify.com/v1/playlists/{}/tracks", playlist_id);
        let token = self.require_token()?;

        let track_uri = if song_id.starts_with("spotify:track:") {
            song_id.to_string()
        } else {
            format!("spotify:track:{}", song_id)
        };

        let body = json!({
            "uris": [track_uri],
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .bearer_auth(token)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status_code = response.status().as_u16();
            let code_text = response.text().await?;
            Err(Self::map_error_status(status_code, code_text))
        }
    }

    // --- Function 6: Remove song from an existing playlist ---
    pub async fn remove_song_from_playlist(
        &self,
        playlist_id: &str,
        song_id: &str,
    ) -> Result<(), SpotMpError> {
        let url = format!("https://api.spotify.com/v1/playlists/{}/tracks", playlist_id);
        let token = self.require_token()?;

        let track_uri = if song_id.starts_with("spotify:track:") {
            song_id.to_string()
        } else {
            format!("spotify:track:{}", song_id)
        };

        let body = json!({
            "tracks": [
                {
                    "uri": track_uri,
                }
            ]
        });

        let response = self
            .client
            .delete(&url)
            .json(&body)
            .bearer_auth(token)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status_code = response.status().as_u16();
            let code_text = response.text().await?;
            Err(Self::map_error_status(status_code, code_text))
        }
    }

    // --- Function 7: Create playlist with an optional initial song ---
    pub async fn create_playlist(
        &self,
        playlist_name: &str,
        public: bool,
        song_to_add: Option<&Song>,
    ) -> Result<Playlist, SpotMpError> {
        let url = "https://api.spotify.com/v1/me/playlists";
        let token = self.require_token()?;

        let body = json!({
            "name": playlist_name,
            "public": public,
        });

        let response = self
            .client
            .post(url)
            .json(&body)
            .bearer_auth(token)
            .send()
            .await?;

        if response.status().is_success() {
            let playlist_response = response.json::<PlaylistItem>().await?;

            let mut new_playlist = Playlist::new(
                Vec::new(),
                true,
                playlist_response.name,
                PlaylistOrigin::Spotify(SpotifyPlaylistData {
                    id_playlist: playlist_response.id,
                }),
            );

            if let Some(song) = song_to_add {
                self.add_song_to_playlist(&new_playlist.id_playlist(), song.song_data())
                    .await?;
                new_playlist.add_song(song.clone());
            }

            Ok(new_playlist)
        } else {
            let status_code = response.status().as_u16();
            let code_text = response.text().await?;
            Err(Self::map_error_status(status_code, code_text))
        }
    }

    // --- Function 8: Unfollow (delete) a Spotify playlist ---
    pub async fn unfollow_playlist(&self, playlist_id: &str) -> Result<(), SpotMpError> {
        let url = format!(
            "https://api.spotify.com/v1/playlists/{}/followers",
            playlist_id
        );
        let token = self.require_token()?;

        let response = self
            .client
            .delete(&url)
            .bearer_auth(token)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status_code = response.status().as_u16();
            let code_text = response.text().await?;
            Err(Self::map_error_status(status_code, code_text))
        }
    }
}
