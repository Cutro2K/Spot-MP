use std::path::PathBuf;

use log::{info};
use crate::{domain::{playlist::{Playlist, PlaylistOrigin, SpotifyPlaylistData}, song::Song, spot_mp_errors::SpotMpError}, services::data_storage::{db_controller::DBController, file_manager::FileManager}, state_store::app::action::{Action, LocalAction}};

pub struct LocalController {
    file_manager: FileManager,
    db_controller: DBController,
}

impl LocalController {
    pub fn new() -> Result<Self, SpotMpError> {
        info!("Initializing the File Manager...");
        let f_manager = FileManager::new();

        info!("Initializing the Data Base controller...");
        let db_cont = DBController::new(f_manager.config_folder().to_str().expect("The config folder name is unvailable!"))?;

        Ok(Self {
            file_manager: f_manager,
            db_controller: db_cont,
        })
    }  

    // --- Fetch Funcions ---
    pub fn fetch_playlists(&self) -> Action {
        match self.db_controller.get_playlists() {
            Ok(playlists) => {
                Action::Local(LocalAction::FetchedPlaylists(playlists))
            },
            Err(e) => {
                Action::UnexpectedErrorHappened(format!("Error Happened fetching local playlists: {}", e))
            },
        }
    }

    pub fn fetch_local_songs(&self) -> Action {
        match self.file_manager.get_local_songs() {
            Ok(songs) => {
                Action::Local(LocalAction::FetchedSongs(songs))
            },
            Err(e) => {
                Action::UnexpectedErrorHappened(format!("Error happened fetching local songs: {}", e))
            }
        }
    }

    // --- Auxiliar Functions ---
    pub fn validate_file_existence(&self, file_name: &str) -> bool {
        self.file_manager.check_file_existence(file_name)
    }

    // --- Modify Functions ---
    pub fn create_playlist(&mut self, playlist_name: &str) -> Action {
        let playlist_id = match self.db_controller.create_playlist(playlist_name) {
            Ok(id) => id,
            Err(e) => {
                return Action::UnexpectedErrorHappened(format!("Failed trying to create a local playlist: {}", e));
            }
        };
        let new_playlist = Playlist::new(Vec::new(), true, playlist_name.to_string(), PlaylistOrigin::Spotify(SpotifyPlaylistData {id_playlist: playlist_id.to_string()}));
        
        Action::Local(LocalAction::CreatedPlaylist(new_playlist))
    }

    pub fn delete_playlist(&mut self, playlist_id: i64) -> Action {
        match self.db_controller.delete_playlist(playlist_id) {
            Ok(_) => {
                Action::Local(LocalAction::DeletedPlaylist(playlist_id))
            },
            Err(e) => {
                Action::UnexpectedErrorHappened(format!("Failed trying to delet a local playlist: {}", e))
            }
        }
    }

    pub fn add_song_to_playlist(&mut self, playlist_id: i64, mut song: Song) -> Action {        
        match self.db_controller.add_song_to_playlist(playlist_id, song.clone()) {
            Ok(song_id) => {
                song.set_id_song(song_id);
                Action::Local(LocalAction::AddedSongTo(song, playlist_id))
            },
            Err(e) => {
                Action::UnexpectedErrorHappened(format!("Failed to add {}, to a playlist. Error: {}", song.song_name(), e))
            }
        }
    }

    pub fn remove_song_from_playlist(&mut self, playlist_id: i64, song_id: i64) -> Action {
        match self.db_controller.remove_song_from_playlist(playlist_id, song_id) {
            Ok(_) => {
                Action::Local(LocalAction::ErasedSongFrom(song_id, playlist_id))
            },
            Err(e) => {
                Action::UnexpectedErrorHappened(format!("Failed to erase a song from a playlist: {}", e))
            },
        }
    }

    // --- Getters --- 
    pub fn get_spot_mp_folder(&self) -> PathBuf {
        self.file_manager.config_folder()
    }
}   


