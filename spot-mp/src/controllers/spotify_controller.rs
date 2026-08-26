use log::{error, info};
use crate::{domain::{song::Song, spot_mp_errors::{SpotMpError::{self}, Status}}, services::spotify_api::spotify_api_client::SpotifyApiClient, state_store::app::action::{Action, FunctionToRepeat, SpotifyAction}};

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
    
    pub async fn login(&mut self, client_id: String) -> Result<String, SpotMpError> {
        // Try to login with given client id...
        info!("Login in to spotify with client id: {}...", client_id);
        let access_token = match self.client.login(&client_id).await {
            Ok(token) => token,
            Err(e) => {
                error!("Spotify Error (Login): {}", e);
                return Err(e);
            }
        };
        
        
        self.client_id = Some(client_id);
        Ok(access_token)
    }

    pub fn logout(&mut self) -> Action {
        self.client.logout();
        Action::Spotify(SpotifyAction::LoggedOut)
    }

    pub async fn renew_token(&mut self) -> Result<String, SpotMpError> {
        self.client.renew_token(&self.client_id.clone().unwrap()).await
    }

    pub async fn fetch_playlists(&self) -> Action {
        if !self.logged_in {
            return Action::NoOp;
        }
        if self.client_id.is_none() {
            error!("Tried to fetch playlist and albums when the user is logged out!");
            return Action::Spotify(SpotifyAction::UserIsLoggedOut);
        }

        let playlists = match self.client.import_liked_albums_and_playlist(&self.client_id.clone().unwrap()).await {
            Ok(playlists) => playlists,
            Err(e) => match e {
                SpotMpError::SpotifyErrorStatus(Status::Unauthorized(_)) => {
                    return Action::Spotify(SpotifyAction::ExpiredToken(FunctionToRepeat::FetchedPlaylists));
                },
                _ => {
                    error!("Spotify Error (fetch_playlists): {}", e);
                    return Action::UnexpectedErrorHappened("Failed to import liked albums!".to_string());
                },
            }
        };
            
        Action::Spotify(SpotifyAction::FetchedPlaylists(playlists))
    }

    pub async fn search_query(&self, query: &str) -> Action {
        if !self.logged_in {
            return Action::NoOp;
        }
        if self.client_id.is_none() {
            error!("Tried to proccess a query when the user is logged out!");
            return Action::Spotify(SpotifyAction::UserIsLoggedOut);
        }

        match self.client.search_query(query).await {
            Ok(result) => {
                Action::Spotify(SpotifyAction::QueryResult(result)) 
            }
            Err(e) => match e {
                SpotMpError::SpotifyErrorStatus(Status::Unauthorized(_)) => {
                    Action::Spotify(SpotifyAction::ExpiredToken(FunctionToRepeat::SearchQuery(query.to_string())))
                },
                _ => {
                    error!("Spotify Error (search a query): {}", e);
                    Action::UnexpectedErrorHappened("Failed to proccess the query!".to_string())
                },
            }
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
            Ok(_) => {
                Action::Spotify(SpotifyAction::AddedSongTo(song, playlist_id.to_string()))   
            },
            Err(e) => {
                match e {
                    SpotMpError::SpotifyErrorStatus(Status::Unauthorized(_)) => {
                        Action::Spotify(SpotifyAction::ExpiredToken(FunctionToRepeat::AddSongTo(song, playlist_id.to_string())))
                    },
                    _ => {
                        error!("Spotify Error (add song to playlist): {}", e);
                        Action::UnexpectedErrorHappened("Failed to add song to spotify playlist!".to_string())
                    },
                }
            },
        }
    }

    pub async fn remove_song_from_playlist(&self, playlist_id: &str, song: Song) -> Action {
        let song_id = song.song_data();

        match self.client.remove_song_to_playlist(playlist_id, song_id).await {
            Ok(_) => {
                Action::Spotify(SpotifyAction::RemovedSongFrom(song, playlist_id.to_string()))
            },
            Err(e) => {
                match e {
                    SpotMpError::SpotifyErrorStatus(Status::Unauthorized(_)) => {
                        Action::Spotify(SpotifyAction::ExpiredToken(FunctionToRepeat::RemoveSongTo(song, playlist_id.to_string())))
                    },
                    _ => {
                        error!("Spotify Error (remove song from playlist): {}", e);
                        Action::UnexpectedErrorHappened("Failed to remove song from spotify playlist!".to_string())
                    },
                }
            }
        }
    }

    pub async fn create_playlist(&self, playlist_name: &str, public: bool, opt_song: Option<Song>) -> Action {
        if let Some(song) = opt_song {
            let song_data = song.song_data();
            match self.client.create_playlist(playlist_name, public, Some(song_data)).await {
                Ok(playlist) => {
                    Action::Spotify(SpotifyAction::PlaylistCreatedWith(playlist, Some(song)))
                },
                Err(e) => {
                    match e {
                        SpotMpError::SpotifyErrorStatus(Status::Unauthorized(_)) => {
                            Action::Spotify(SpotifyAction::ExpiredToken(FunctionToRepeat::CreatePlaylistWith(playlist_name.to_string(), public, Some(song))))
                        },
                        _ => {
                            error!("Spotify Error (create playlist): {}", e);
                            Action::UnexpectedErrorHappened("Failed to create the playlist!".to_string())
                        },
                    }
                }
            }
        }
        else {
            match self.client.create_playlist(playlist_name, public, None).await {
                Ok(playlist) => {
                    Action::Spotify(SpotifyAction::PlaylistCreatedWith(playlist, None))
                },
                Err(e) => {
                    match e {
                        SpotMpError::SpotifyErrorStatus(Status::Unauthorized(_)) => {
                            Action::Spotify(SpotifyAction::ExpiredToken(FunctionToRepeat::CreatePlaylistWith(playlist_name.to_string(), public, None)))
                        },
                        _ => {
                            error!("Spotify Error (create playlist): {}", e);
                            Action::UnexpectedErrorHappened("Failed to create the playlist!".to_string())
                        },
                    }
                }
            }
        }
    }
}
