use crate::domain::{
    playlist::{LocalPlaylistData, Playlist, PlaylistOrigin},
    song::{LocalSongData, Song, SongOrigin, SpotifySongData},
    spot_mp_errors::SpotMpError,
};
use sqlite::{Connection, Error};

pub struct DBController {
    connection: Connection,
}

impl DBController {
    // --- Useful Functions ---
    // Check playlist existence
    fn check_playlist_existence(&self, playlist_id: i64) -> Result<bool, SpotMpError> {
        let query = "SELECT 1 FROM Playlist WHERE ID_playlist = (?);";
        let mut statement = self.connection.prepare(query)?;
        statement.bind((1, playlist_id))?;

        if let Ok(sqlite::State::Row) = statement.next() {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    // --- Function 1: Constructor ---
    // This function creates or connects to a database, depending on database existence.
    // If you use :memory: for db_path you will create a temporary database in RAM,
    // which is useful for testing.
    pub fn new(db_path: &str) -> Result<Self, Error> {
        let connection = sqlite::open(db_path)?;

        // Enable foreign keys so ON DELETE CASCADE works in SQLite, and create tables if they don't exist
        let init_query = "
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS Playlist (
                ID_playlist INTEGER PRIMARY KEY AUTOINCREMENT,
                Playlist_name TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS Song (
                ID_song INTEGER PRIMARY KEY AUTOINCREMENT,
                song_name TEXT NOT NULL,
                artist TEXT NOT NULL DEFAULT 'Unknown Artist',
                album TEXT NOT NULL DEFAULT 'Unknown Album',
                release_year INTEGER,
                duration INTEGER,
                is_local BOOLEAN,
                file_uri TEXT UNIQUE,
                img_path TEXT
            );

            CREATE TABLE IF NOT EXISTS Playlist_Song (
                ID_playlist INTEGER,
                ID_song INTEGER,
                PRIMARY KEY (ID_playlist, ID_song),
                FOREIGN KEY (ID_playlist) REFERENCES Playlist(ID_playlist) ON DELETE CASCADE,
                FOREIGN KEY (ID_song) REFERENCES Song(ID_song) ON DELETE CASCADE
            );
        ";

        connection.execute(init_query)?;

        Ok(Self { connection })
    }

    // --- Function 2: create_playlist ---
    // Creates a new playlist, saves it in the database and returns the playlist_id
    pub fn create_playlist(&self, name: &str) -> Result<i64, SpotMpError> {
        let query = "INSERT INTO Playlist (Playlist_name) VALUES (?) RETURNING ID_playlist;";
        let mut statement = self.connection.prepare(query)?;
        statement.bind((1, name))?;

        if let Ok(sqlite::State::Row) = statement.next() {
            let new_id: i64 = statement.read::<i64, _>(0)?;
            Ok(new_id)
        } else {
            Err(SpotMpError::SQLite(sqlite::Error {
                code: None,
                message: Some("Could not retrieve the ID of the new playlist".to_string()),
            }))
        }
    }

    // --- Function 3: delete_playlist ---
    // Deletes a playlist from the database
    pub fn delete_playlist(&self, playlist_id: i64) -> Result<(), SpotMpError> {
        let query = "DELETE FROM Playlist WHERE ID_playlist = (?);";
        let mut statement = self.connection.prepare(query)?;
        statement.bind((1, playlist_id))?;

        if let Ok(sqlite::State::Done) = statement.next() {
            Ok(())
        } else {
            Err(SpotMpError::SQLite(sqlite::Error {
                code: None,
                message: Some(format!(
                    "Could not delete the playlist with {playlist_id} ID."
                )),
            }))
        }
    }

    // --- Function 4: add_song_to_playlist ---
    // Adds a song to a playlist, reusing the existing Song record on file_uri conflict
    pub fn add_song_to_playlist(&self, playlist_id: i64, song: Song) -> Result<i64, SpotMpError> {
        if !self.check_playlist_existence(playlist_id)? {
            return Err(SpotMpError::SQLite(sqlite::Error {
                code: None,
                message: Some(format!("The playlist with {playlist_id} ID doesn't exist.")),
            }));
        }

        // --- Query 1: Upsert the song into the database so shared songs across playlists don't fail UNIQUE(file_uri) ---
        let song_query = "
            INSERT INTO Song (song_name, artist, album, release_year, duration, is_local, file_uri, img_path)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(file_uri) DO UPDATE SET
                song_name = excluded.song_name,
                artist = excluded.artist,
                album = excluded.album,
                release_year = excluded.release_year,
                duration = excluded.duration,
                img_path = excluded.img_path
            RETURNING ID_song;
        ";

        let mut song_statement = self.connection.prepare(song_query)?;

        song_statement.bind((1, song.song_name()))?;
        song_statement.bind((2, song.artist()))?;
        song_statement.bind((3, song.album()))?;
        match song.release_year() {
            Some(year) => song_statement.bind((4, year as i64))?,
            None => song_statement.bind((4, sqlite::Value::Null))?,
        }
        song_statement.bind((5, song.duration()))?;
        song_statement.bind((6, if song.is_local() { 1 } else { 0 }))?;

        match &song.song() {
            SongOrigin::Local(song_data) => {
                song_statement.bind((7, song_data.file_direction.as_str()))?;
            }
            SongOrigin::Spotify(song_data) => {
                song_statement.bind((7, song_data.spotify_id.as_str()))?;
            }
        }
        song_statement.bind((8, song.img_path()))?;

        let song_id = if let Ok(sqlite::State::Row) = song_statement.next() {
            song_statement.read::<i64, _>(0)?
        } else {
            return Err(SpotMpError::SQLite(sqlite::Error {
                code: None,
                message: Some("Failed to retrieve the generated ID_song".to_string()),
            }));
        };

        // --- Query 2: Link to the playlist ---
        let link_query = "INSERT OR IGNORE INTO Playlist_Song (ID_playlist, ID_song) VALUES (?, ?)";
        let mut link_statement = self.connection.prepare(link_query)?;

        link_statement.bind((1, playlist_id))?;
        link_statement.bind((2, song_id))?;

        if let Ok(sqlite::State::Done) = link_statement.next() {
            Ok(song_id)
        } else {
            Err(SpotMpError::SQLite(sqlite::Error {
                code: None,
                message: Some("Failed to link the song to the playlist".to_string()),
            }))
        }
    }

    // --- Function 5: get_playlists ---
    // Gets all the playlists and songs from the database
    pub fn get_playlists(&self) -> Result<Vec<Playlist>, SpotMpError> {
        let playlist_query = "SELECT ID_playlist, Playlist_name FROM Playlist;";
        let mut playlist_statement = self.connection.prepare(playlist_query)?;
        let mut playlists: Vec<Playlist> = Vec::new();

        while let Ok(sqlite::State::Row) = playlist_statement.next() {
            let playlist_id: i64 = playlist_statement.read::<i64, _>(0)?;
            let playlist_name: String = playlist_statement.read::<String, _>(1)?;

            let songs: Vec<Song> = self.get_songs_from_playlists(playlist_id)?;

            let playlist = Playlist::new(
                songs,
                true,
                playlist_name,
                None,
                PlaylistOrigin::Local(LocalPlaylistData {
                    id_playlist: playlist_id,
                }),
            );

            playlists.push(playlist);
        }

        Ok(playlists)
    }

    fn get_songs_from_playlists(&self, playlist_id: i64) -> Result<Vec<Song>, SpotMpError> {
        let mut songs: Vec<Song> = Vec::new();

        let song_query = "
            SELECT S.ID_song, S.song_name, S.artist, S.album, S.release_year, S.duration, S.is_local, S.file_uri, S.img_path
            FROM Song S
            INNER JOIN Playlist_Song PS ON S.ID_song = PS.ID_song
            WHERE PS.ID_playlist = ?;
        ";

        let mut song_statement = self.connection.prepare(song_query)?;
        song_statement.bind((1, playlist_id))?;

        while let Ok(sqlite::State::Row) = song_statement.next() {
            let song_id = song_statement.read::<i64, _>(0)?;
            let song_name = song_statement.read::<String, _>(1)?;
            let artist = song_statement.read::<String, _>(2)?;
            let album = song_statement.read::<String, _>(3)?;
            let release_year = song_statement
                .read::<Option<i64>, _>(4)?
                .map(|y| y as i32);
            let duration = song_statement.read::<i64, _>(5)?;
            let local = !matches!(song_statement.read::<i64, _>(6)?, 0);
            let file_uri = song_statement.read::<String, _>(7)?;
            let origin = if local {
                SongOrigin::Local(LocalSongData {
                    file_direction: file_uri,
                })
            } else {
                SongOrigin::Spotify(SpotifySongData {
                    spotify_id: file_uri,
                })
            };
            let path = song_statement.read::<String, _>(8)?;

            let song = Song::with_metadata(
                Some(song_id),
                song_name,
                artist,
                album,
                release_year,
                duration,
                path,
                origin,
            );

            songs.push(song);
        }

        Ok(songs)
    }

    pub fn remove_song_from_playlist(
        &self,
        playlist_id: i64,
        song_id: i64,
    ) -> Result<(), SpotMpError> {
        let query = "DELETE FROM Playlist_Song WHERE ID_playlist = ? AND ID_song = ?";
        let mut stmt = self.connection.prepare(query)?;

        stmt.bind((1, playlist_id))?;
        stmt.bind((2, song_id))?;

        if let Ok(sqlite::State::Done) = stmt.next() {
            Ok(())
        } else {
            Err(SpotMpError::SQLite(sqlite::Error {
                code: None,
                message: Some(format!(
                    "Could not remove the song with the id {} in playlist with {} ID.",
                    song_id, playlist_id
                )),
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::song::LocalSongData;

    fn setup_memory_db() -> DBController {
        DBController::new(":memory:").expect("Failed to initialize the in-memory database.")
    }

    fn setup_playlist(db: &DBController) -> i64 {
        let playlist_name = "My favorite Songs";
        let result = db.create_playlist(playlist_name);
        assert!(result.is_ok(), "Failed to create the playlist in the database.");
        result.unwrap()
    }

    #[test]
    fn test_in_memory_database_creation() {
        let db = setup_memory_db();
        let query = "SELECT count(name) FROM sqlite_master WHERE type='table' AND name='Playlist';";
        let mut table_exists = false;

        db.connection
            .iterate(query, |pairs| {
                for &(_column, value) in pairs.iter() {
                    if value == Some("1") {
                        table_exists = true;
                    }
                }
                true
            })
            .expect("Error querying sqlite_master.");

        assert!(
            table_exists,
            "The 'Playlist' table was not created during initialization."
        );
    }

    #[test]
    fn test_create_playlist() {
        let db = setup_memory_db();
        let playlist_name = "My Favorite Songs";
        let result = db.create_playlist(playlist_name);

        assert!(result.is_ok(), "Failed to create the playlist in the database.");
        let new_id = result.unwrap();
        assert_eq!(new_id, 1, "The first created playlist should have ID 1.");
    }

    #[test]
    fn test_delete_playlist() {
        let db = setup_memory_db();
        let id_playlist: i64 = setup_playlist(&db);
        let result_deletion = db.delete_playlist(id_playlist);
        assert!(
            result_deletion.is_ok(),
            "Failed to delete the playlist in the database."
        );
    }

    #[test]
    fn test_add_song_to_playlist() {
        let db = setup_memory_db();
        let id_playlist: i64 = setup_playlist(&db);

        let song = Song::new(
            None,
            "SongFav".to_string(),
            60,
            "".to_string(),
            SongOrigin::Local(LocalSongData {
                file_direction: "somewhere".to_string(),
            }),
        );

        assert!(
            db.add_song_to_playlist(id_playlist, song).is_ok(),
            "Failed to add a song to a playlist in the database."
        );
    }

    #[test]
    fn test_add_same_song_to_multiple_playlists() {
        let db = setup_memory_db();
        let p1 = db.create_playlist("Playlist 1").unwrap();
        let p2 = db.create_playlist("Playlist 2").unwrap();

        let song = Song::with_metadata(
            None,
            "Shared Song".to_string(),
            "Queen".to_string(),
            "A Night at the Opera".to_string(),
            Some(1975),
            354,
            "".to_string(),
            SongOrigin::Local(LocalSongData {
                file_direction: "/music/shared.mp3".to_string(),
            }),
        );

        let id1 = db.add_song_to_playlist(p1, song.clone()).expect("Failed to add to p1");
        let id2 = db.add_song_to_playlist(p2, song).expect("Failed to add to p2");
        assert_eq!(id1, id2, "Reused song should keep the same ID_song");
    }

    #[test]
    fn test_get_playlists() {
        let db = setup_memory_db();
        let playlist_name = "Rock Classics";
        let playlist_id = db
            .create_playlist(playlist_name)
            .expect("Failed to create the initial playlist");

        let song1 = Song::new(
            None,
            "Bohemian Rhapsody".to_string(),
            354,
            "".to_string(),
            SongOrigin::Local(LocalSongData {
                file_direction: "/music/queen/bohemian.mp3".to_string(),
            }),
        );

        let song2 = Song::new(
            None,
            "Hotel California".to_string(),
            390,
            "https://spotify.com/img/...".to_string(),
            SongOrigin::Spotify(SpotifySongData {
                spotify_id: "spotify:track:40riOy7x9W7GXjyhghGhw9".to_string(),
            }),
        );

        db.add_song_to_playlist(playlist_id, song1)
            .expect("Failed to add song1");
        db.add_song_to_playlist(playlist_id, song2)
            .expect("Failed to add song2");

        let fetch_result = db.get_playlists();
        assert!(fetch_result.is_ok(), "Failed to execute the get_playlists query");

        let playlists = fetch_result.unwrap();
        assert_eq!(
            playlists.len(),
            1,
            "There should be exactly 1 playlist in the database"
        );

        let fetched_playlist = &playlists[0];
        assert_eq!(
            fetched_playlist.playlist_name(),
            playlist_name,
            "The fetched playlist name does not match"
        );

        assert_eq!(
            fetched_playlist.get_duration(),
            744,
            "The total duration does not match. The songs were not correctly hydrated when fetching the playlist."
        );
    }
}
