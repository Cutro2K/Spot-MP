use std::path::PathBuf;

use log::info;

use crate::{
    domain::{
        playlist::{LocalPlaylistData, Playlist, PlaylistOrigin},
        song::Song,
        spot_mp_errors::SpotMpError,
    },
    services::data_storage::{
        config_storage::ConfigStorage, cover_cache::CoverArtCache, db_controller::DBController,
        song_scanner::SongScanner,
    },
    state_store::app::action::{Action, LocalAction},
};

pub struct LocalController {
    config_storage: ConfigStorage,
    song_scanner: SongScanner,
    db_controller: DBController,
}

impl LocalController {
    pub fn new() -> Result<Self, SpotMpError> {
        info!("Initializing configuration storage...");
        Self::from_config_storage(ConfigStorage::new())
    }

    pub fn with_config_folder(config_folder: PathBuf) -> Result<Self, SpotMpError> {
        Self::from_config_storage(ConfigStorage::with_config_folder(config_folder))
    }

    fn from_config_storage(config_storage: ConfigStorage) -> Result<Self, SpotMpError> {
        info!("Initializing cover cache and local song scanner...");
        let cover_cache = CoverArtCache::new(config_storage.cache_folder());
        let song_scanner = SongScanner::new(config_storage.music_folder(), cover_cache);

        info!("Initializing the Data Base controller...");
        let db_file_path = config_storage.config_folder().join("spot-mp.db");
        let db_cont = DBController::new(
            db_file_path
                .to_str()
                .expect("The database file path is invalid UTF-8!"),
        )?;

        Ok(Self {
            config_storage,
            song_scanner,
            db_controller: db_cont,
        })
    }

    // --- Fetch Functions ---
    pub fn fetch_playlists(&self) -> Action {
        match self.db_controller.get_playlists() {
            Ok(playlists) => Action::Local(LocalAction::FetchedPlaylists(playlists)),
            Err(e) => Action::UnexpectedErrorHappened(format!(
                "Error happened fetching local playlists: {}",
                e
            )),
        }
    }

    pub fn fetch_local_songs(&self) -> Action {
        match self.song_scanner.get_local_songs() {
            Ok(songs) => Action::Local(LocalAction::FetchedSongs(songs)),
            Err(e) => Action::UnexpectedErrorHappened(format!(
                "Error happened fetching local songs: {}",
                e
            )),
        }
    }

    // --- Auxiliary Functions ---
    pub fn validate_file_existence(&self, file_name: &str) -> bool {
        self.song_scanner.check_file_existence(file_name)
    }

    // --- Modify Functions ---
    pub fn set_music_folder(&mut self, new_folder: PathBuf) -> Result<(), SpotMpError> {
        self.config_storage.set_music_folder(new_folder.clone())?;
        self.song_scanner.set_music_folder(new_folder);
        Ok(())
    }

    pub fn create_playlist(&mut self, playlist_name: &str) -> Action {
        let playlist_id = match self.db_controller.create_playlist(playlist_name) {
            Ok(id) => id,
            Err(e) => {
                return Action::UnexpectedErrorHappened(format!(
                    "Failed trying to create a local playlist: {}",
                    e
                ));
            }
        };
        let new_playlist = Playlist::new(
            Vec::new(),
            true,
            playlist_name.to_string(),
            None,
            PlaylistOrigin::Local(LocalPlaylistData {
                id_playlist: playlist_id,
            }),
        );

        Action::Local(LocalAction::CreatedPlaylist(new_playlist))
    }

    pub fn delete_playlist(&mut self, playlist_id: i64) -> Action {
        match self.db_controller.delete_playlist(playlist_id) {
            Ok(_) => Action::Local(LocalAction::DeletedPlaylist(playlist_id)),
            Err(e) => Action::UnexpectedErrorHappened(format!(
                "Failed trying to delete a local playlist: {}",
                e
            )),
        }
    }

    pub fn add_song_to_playlist(&mut self, playlist_id: i64, mut song: Song) -> Action {
        if song.is_local() && !self.validate_file_existence(song.song_data()) {
            return Action::UnexpectedErrorHappened(format!(
                "The file for {} is no longer available on disk.",
                song.song_name()
            ));
        }

        match self
            .db_controller
            .add_song_to_playlist(playlist_id, song.clone())
        {
            Ok(song_id) => {
                song.set_id_song(song_id);
                Action::Local(LocalAction::AddedSongTo(song, playlist_id))
            }
            Err(e) => Action::UnexpectedErrorHappened(format!(
                "Failed to add {}, to a playlist. Error: {}",
                song.song_name(),
                e
            )),
        }
    }

    pub fn remove_song_from_playlist(&mut self, playlist_id: i64, song_id: i64) -> Action {
        match self
            .db_controller
            .remove_song_from_playlist(playlist_id, song_id)
        {
            Ok(_) => Action::Local(LocalAction::ErasedSongFrom(song_id, playlist_id)),
            Err(e) => Action::UnexpectedErrorHappened(format!(
                "Failed to erase a song from a playlist: {}",
                e
            )),
        }
    }

    // --- Getters ---
    pub fn get_spot_mp_folder(&self) -> PathBuf {
        self.config_storage.config_folder()
    }

    pub fn get_music_folder(&self) -> PathBuf {
        self.song_scanner.song_folder()
    }
}
