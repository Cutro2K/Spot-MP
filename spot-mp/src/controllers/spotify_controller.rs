use log::{error, info};

use crate::{
    domain::{
        auth_models::WebApiData,
        song::Song,
        spot_mp_errors::{SpotMpError, Status},
    },
    services::spotify_api::spotify_api_client::SpotifyApiClient,
    state_store::app::action::{Action, FunctionToRepeat, SpotifyAction},
};

pub struct SpotifyController {
    client_id: Option<String>,
    logged_in: bool,
    client: SpotifyApiClient,
}

impl Default for SpotifyController {
    fn default() -> Self {
        Self::new()
    }
}

impl SpotifyController {
    pub fn new() -> Self {
        Self {
            client_id: None,
            logged_in: false,
            client: SpotifyApiClient::new(),
        }
    }

    pub fn restore_session(&mut self, client_id: String, web_api: WebApiData) {
        self.client_id = Some(client_id.clone());
        self.client
            .restore_session(client_id, web_api.access_token, web_api.refresh_token);
        self.logged_in = true;
    }

    pub fn client_id(&self) -> Option<String> {
        self.client_id.clone()
    }

    pub fn tokens(&self) -> Option<WebApiData> {
        let access_token = self.client.access_token()?;
        let refresh_token = self.client.refresh_token()?;
        Some(WebApiData {
            access_token,
            refresh_token,
        })
    }

    pub async fn login(&mut self, client_id: String) -> Result<(), SpotMpError> {
        info!("Logging in to Spotify with client id: {}...", client_id);
        if let Err(e) = self.client.login(&client_id).await {
            error!("Spotify Error (Login): {}", e);
            return Err(e);
        }
        self.client_id = Some(client_id);
        self.logged_in = true;
        Ok(())
    }

    pub fn logout(&mut self) -> Action {
        self.client.logout();
        self.logged_in = false;
        self.client_id = None;
        Action::Spotify(SpotifyAction::LoggedOut)
    }

    pub async fn fetch_playlists(&mut self) -> Action {
        if !self.logged_in {
            return Action::NoOp;
        }
        if self.client_id.is_none() {
            error!("Tried to fetch playlists when the user is logged out!");
            return Action::Spotify(SpotifyAction::UserIsLoggedOut);
        }

        let playlists = match self.client.import_liked_playlist().await {
            Ok(playlists) => playlists,
            Err(e) => match e {
                SpotMpError::SpotifyErrorStatus(Status::Unauthorized(_)) => {
                    return Action::Spotify(SpotifyAction::ExpiredToken(
                        FunctionToRepeat::FetchedPlaylists,
                    ));
                }
                _ => {
                    error!("Spotify Error (fetch_playlists): {}", e);
                    return Action::UnexpectedErrorHappened(
                        "Failed to import liked playlists!".to_string(),
                    );
                }
            },
        };

        Action::Spotify(SpotifyAction::FetchedPlaylists(playlists))
    }

    pub async fn fetch_albums(&self) -> Action {
        if !self.logged_in {
            return Action::NoOp;
        }
        if self.client_id.is_none() {
            error!("Tried to fetch albums when the user is logged out!");
            return Action::Spotify(SpotifyAction::UserIsLoggedOut);
        }

        let albums = match self.client.import_liked_albums().await {
            Ok(albums) => albums,
            Err(e) => match e {
                SpotMpError::SpotifyErrorStatus(Status::Unauthorized(_)) => {
                    return Action::Spotify(SpotifyAction::ExpiredToken(
                        FunctionToRepeat::FetchedAlbums,
                    ));
                }
                _ => {
                    error!("Spotify Error (fetch_albums): {}", e);
                    return Action::UnexpectedErrorHappened(
                        "Failed to import liked albums!".to_string(),
                    );
                }
            },
        };

        Action::Spotify(SpotifyAction::FetchedAlbums(albums))
    }

    pub async fn search_query(&self, query: &str) -> Action {
        if !self.logged_in {
            return Action::NoOp;
        }
        if self.client_id.is_none() {
            error!("Tried to process a query when the user is logged out!");
            return Action::Spotify(SpotifyAction::UserIsLoggedOut);
        }

        match self.client.search_query(query).await {
            Ok(result) => Action::Spotify(SpotifyAction::QueryResult(result)),
            Err(e) => match e {
                SpotMpError::SpotifyErrorStatus(Status::Unauthorized(_)) => Action::Spotify(
                    SpotifyAction::ExpiredToken(FunctionToRepeat::SearchQuery(query.to_string())),
                ),
                _ => {
                    error!("Spotify Error (search a query): {}", e);
                    Action::UnexpectedErrorHappened("Failed to process the query!".to_string())
                }
            },
        }
    }

    pub async fn add_song_to_playlist(&self, playlist_id: &str, song: Song) -> Action {
        if !self.logged_in {
            return Action::NoOp;
        }
        if self.client_id.is_none() {
            error!("Tried to add a song to a playlist when the user is logged out!");
            return Action::Spotify(SpotifyAction::UserIsLoggedOut);
        }

        let song_id = song.song_data();
        match self.client.add_song_to_playlist(playlist_id, song_id).await {
            Ok(_) => Action::Spotify(SpotifyAction::AddedSongTo(song, playlist_id.to_string())),
            Err(e) => match e {
                SpotMpError::SpotifyErrorStatus(Status::Unauthorized(_)) => {
                    Action::Spotify(SpotifyAction::ExpiredToken(FunctionToRepeat::AddSongTo(
                        song,
                        playlist_id.to_string(),
                    )))
                }
                _ => {
                    error!("Spotify Error (add song to playlist): {}", e);
                    Action::UnexpectedErrorHappened(
                        "Failed to add song to Spotify playlist!".to_string(),
                    )
                }
            },
        }
    }

    pub async fn remove_song_from_playlist(&self, playlist_id: &str, song: Song) -> Action {
        if !self.logged_in {
            return Action::NoOp;
        }
        if self.client_id.is_none() {
            error!("Tried to remove a song from a playlist when the user is logged out!");
            return Action::Spotify(SpotifyAction::UserIsLoggedOut);
        }

        let song_id = song.song_data();

        match self
            .client
            .remove_song_from_playlist(playlist_id, song_id)
            .await
        {
            Ok(_) => Action::Spotify(SpotifyAction::RemovedSongFrom(
                song,
                playlist_id.to_string(),
            )),
            Err(e) => match e {
                SpotMpError::SpotifyErrorStatus(Status::Unauthorized(_)) => {
                    Action::Spotify(SpotifyAction::ExpiredToken(FunctionToRepeat::RemoveSongTo(
                        song,
                        playlist_id.to_string(),
                    )))
                }
                _ => {
                    error!("Spotify Error (remove song from playlist): {}", e);
                    Action::UnexpectedErrorHappened(
                        "Failed to remove song from Spotify playlist!".to_string(),
                    )
                }
            },
        }
    }

    pub async fn create_playlist(
        &self,
        playlist_name: &str,
        public: bool,
        opt_song: Option<Song>,
    ) -> Action {
        if !self.logged_in {
            return Action::NoOp;
        }
        if self.client_id.is_none() {
            error!("Tried to create a playlist when the user is logged out!");
            return Action::Spotify(SpotifyAction::UserIsLoggedOut);
        }

        match self
            .client
            .create_playlist(playlist_name, public, opt_song.as_ref())
            .await
        {
            Ok(playlist) => {
                Action::Spotify(SpotifyAction::PlaylistCreatedWith(playlist, opt_song))
            }
            Err(e) => match e {
                SpotMpError::SpotifyErrorStatus(Status::Unauthorized(_)) => Action::Spotify(
                    SpotifyAction::ExpiredToken(FunctionToRepeat::CreatePlaylistWith(
                        playlist_name.to_string(),
                        public,
                        opt_song,
                    )),
                ),
                _ => {
                    error!("Spotify Error (create playlist): {}", e);
                    Action::UnexpectedErrorHappened("Failed to create the playlist!".to_string())
                }
            },
        }
    }

    pub async fn delete_playlist(&self, playlist_id: &str) -> Action {
        if !self.logged_in {
            return Action::NoOp;
        }
        if self.client_id.is_none() {
            error!("Tried to delete a playlist when the user is logged out!");
            return Action::Spotify(SpotifyAction::UserIsLoggedOut);
        }

        match self.client.unfollow_playlist(playlist_id).await {
            Ok(_) => Action::Spotify(SpotifyAction::DeletedPlaylist(playlist_id.to_string())),
            Err(e) => match e {
                SpotMpError::SpotifyErrorStatus(Status::Unauthorized(_)) => Action::Spotify(
                    SpotifyAction::ExpiredToken(FunctionToRepeat::DeletePlaylist(
                        playlist_id.to_string(),
                    )),
                ),
                _ => {
                    error!("Spotify Error (delete playlist): {}", e);
                    Action::UnexpectedErrorHappened("Failed to delete the playlist!".to_string())
                }
            },
        }
    }

    pub async fn delete_album(&self, album_id: &str) -> Action {
        if !self.logged_in {
            return Action::NoOp;
        }
        if self.client_id.is_none() {
            error!("Tried to delete an album when the user is logged out!");
            return Action::Spotify(SpotifyAction::UserIsLoggedOut);
        }

        match self.client.unfollow_album(album_id).await {
            Ok(_) => Action::Spotify(SpotifyAction::DeletedAlbum(album_id.to_string())),
            Err(e) => match e {
                SpotMpError::SpotifyErrorStatus(Status::Unauthorized(_)) => Action::Spotify(
                    SpotifyAction::ExpiredToken(FunctionToRepeat::DeleteAlbum(
                        album_id.to_string(),
                    )),
                ),
                _ => {
                    error!("Spotify Error (delete album): {}", e);
                    Action::UnexpectedErrorHappened("Failed to delete the album!".to_string())
                }
            },
        }
    }
}
