use crate::domain::spot_mp_errors::SpotMpError;
use sqlite::{Connection, Error};
use crate::domain::{playlist::{LocalPlaylistData, Playlist, PlaylistOrigin}, song::{LocalSongData, Song, SongOrigin, SpotifySongData}};

pub struct DBController {
    connection: Connection,
}

impl DBController {
    // --- Usefull Functions --- 
    // Check playlist existance 
    fn check_playlist_existance(&self, playlist_id: i64) -> Result<bool, SpotMpError> {

        let query = "SELECT 1 FROM Playlist WHERE ID_playlist = (?);";

        let mut statement = self.connection.prepare(query)?;
        
        statement.bind((1, playlist_id))?;

        if let Ok(sqlite::State::Row) = statement.next() {
            Ok(true)
        }
        else {
            Ok(false)
        }
    }

    // --- Function 1: Constructor ---
    // This function create or connect to a database, depends on the database existance
    // If you use :memory: for db_path you will create a temporaly database in ram
    // this is usefull for testing
    pub fn new(db_path: &str) -> Result<Self, Error>{

        let connection = sqlite::open(db_path)?;

        // This query create the database tables if they don't exist
        let init_query = "
            CREATE TABLE IF NOT EXISTS Playlist (
                ID_playlist INTEGER PRIMARY KEY AUTOINCREMENT,
                Playlist_name TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS Song (
                ID_song INTEGER PRIMARY KEY AUTOINCREMENT,
                song_name TEXT NOT NULL,
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
    // This function create a new playlist, save it in the database and then return the playlist_id
    pub fn create_playlist(&self, name: &str) -> Result<i64, SpotMpError>{
        
        let query = "INSERT INTO Playlist (Playlist_name) VALUES (?) RETURNING ID_playlist;";
        
        let mut statement = self.connection.prepare(query)?;
        
        statement.bind((1, name))?;

        if let Ok(sqlite::State::Row) = statement.next() {
            let new_id: i64 = statement.read::<i64, _>(0)?;
            Ok(new_id)
        }
        else {
            Err(SpotMpError::SQLite(sqlite::Error {
                code: None,
                message: Some("Could not retrieve the ID of the new playlist".to_string()),
            }))
        }
    }

    // --- Function 3: delete_playlist ---
    // This function delete a playlist from the database
    pub fn delete_playlist(&self, playlist_id: i64) -> Result<(), SpotMpError>{       
        
        let query = "DELETE FROM Playlist WHERE ID_playlist = (?);";
        
        let mut statement = self.connection.prepare(query)?;
        
        statement.bind((1, playlist_id))?;
        
        if let Ok(sqlite::State::Done) = statement.next() {
            Ok(())
        }
        else {
            Err(SpotMpError::SQLite(sqlite::Error {
                code: None,
                message: Some(format!("Could not delete the playlist with {playlist_id} ID.")),
            }))
        }
    }

    // --- Function 4: add_song_to_playlist --- 
    // This function add a song to a playlist
    pub fn add_song_to_playlist(&self, playlist_id: i64, song: Song) -> Result<i64, SpotMpError> {
        
        // Checking the existance of the playlist: 
        if !self.check_playlist_existance(playlist_id).unwrap() {
            return Err( SpotMpError::SQLite(sqlite::Error {
                code: None,
                message: Some(format!("The playlist with {playlist_id} ID doesn't exist.")),
            }))
        }
       
        // --- Query 1: Adding the song to the databas if it necesary --- 
        let song_query = "
                    INSERT INTO Song (song_name, duration, is_local, file_uri, img_path)
                    VALUES (?, ?, ?, ?, ?)
                    RETURNING ID_song;
                ";
        
        let mut song_statement = self.connection.prepare(song_query)?;
        
        song_statement.bind((1, song.get_song_name().as_str()))?;
        song_statement.bind((2, song.duration()))?;
        // sqlite doesn't have native boolean so we store 1 or 0
        match song.is_local() {
            true => {
                song_statement.bind((3, 1))?;
            },
            false => {
                song_statement.bind((3, 0))?;
            },
        }
        // Pattern matching to make the correct functionality:  
        match &song.song(){
            SongOrigin::Local(song_data) => {
                song_statement.bind((4, song_data.file_direction.as_str()))?;
            },
            SongOrigin::Spotify(song_data) => {
                song_statement.bind((4, song_data.spotify_id.as_str()))?;
            },
        }
        song_statement.bind((5, song.img_path().as_str()))?;
        
        let song_id;
    
        if let Ok(sqlite::State::Row) = song_statement.next() {
            song_id = song_statement.read::<i64, _>(0)?;
        }
        else {
            return Err(SpotMpError::SQLite(sqlite::Error {
                code: None,
                message: Some("Failed to retrieve the generated ID_song".to_string()),
            }));
        }
        
        // --- Query 2: Link to a playlist ---
        
        let link_query = "INSERT INTO Playlist_Song (ID_playlist, ID_song) VALUES (?, ?)";
        
        let mut link_statement = self.connection.prepare(link_query)?;

        link_statement.bind((1, playlist_id))?;
        link_statement.bind((2, song_id))?;
        
        if let Ok(sqlite::State::Done) = link_statement.next() {
            Ok(song_id)
        }
        else {
            Err(SpotMpError::SQLite(sqlite::Error {
                code: None,
                message: Some("Failed to link the song to the playlist".to_string())
            }))
        }
    }

    // --- Function 5: get_playlists --- 
    // get all the playlist and songs from the database 
    pub fn get_playlists(&self) -> Result<Vec<Playlist>, SpotMpError> {

        // --- Step 1: Get all the playlist from the database and iterate over them ---
        let playlist_query = "SELECT ID_playlist, Playlist_name FROM Playlist;";

        let mut playlist_statement = self.connection.prepare(playlist_query)?;
        
        let mut playlists: Vec<Playlist> = Vec::new();

        while let Ok(sqlite::State::Row) = playlist_statement.next() {
            let playlist_id: i64 = playlist_statement.read::<i64, _>(0)?;
            let playlist_name: String = playlist_statement.read::<String, _>(1)?;
            
            // Fetch all songs: 
            let songs: Vec<Song> = self.get_songs_from_playlists(playlist_id)?;
            
            let playlist = Playlist::new( 
                songs,
                true,
                playlist_name,
                PlaylistOrigin::Local(LocalPlaylistData {id_playlist: playlist_id}),
            );

            playlists.push(playlist);
        }

        Ok(playlists)
    }

    fn get_songs_from_playlists(&self, playlist_id: i64) -> Result<Vec<Song>, SpotMpError> {
        let mut songs: Vec<Song> = Vec::new();
        
        // Preparing the join for fetching the  songs
        let song_query = "
            SELECT S.ID_song, S.song_name, S.duration, S.is_local, S.file_uri, S.img_path
            FROM Song S
            INNER JOIN Playlist_Song PS ON S.ID_song = PS.ID_song
            WHERE PS.ID_playlist = ?;
        ";
        
        let mut song_statement = self.connection.prepare(song_query)?;
        
        song_statement.bind((1, playlist_id))?;
        
        while let Ok(sqlite::State::Row) = song_statement.next() {
            let song_id = song_statement.read::<i64, _>(0)?;
            let song_name = song_statement.read::<String, _>(1)?;
            let duration = song_statement.read::<i64, _>(2)?;
            let local = match song_statement.read::<i64, _>(3)? {
                1 => true,
                0 => false,
                _ => true,
            };
            let origin = match local {
                true => SongOrigin::Local(LocalSongData {file_direction: song_statement.read::<String, _>(4)?}),
                false => SongOrigin::Spotify(SpotifySongData {spotify_id: song_statement.read::<String, _>(4)?}),
            };
            let path = song_statement.read::<String, _>(5)?;

            let song = Song::new(Some(song_id), song_name, duration, path, origin);

            songs.push(song);
        }

        Ok(songs)
    }
}

 
// Testing
#[cfg(test)]
mod tests {
    use crate::domain::song::LocalSongData;
    use super::*;

    // Seting up a RAM db for testing
    fn setup_memory_db() -> DBController {
        DBController::new(":memory:").expect("Failed to initialize the in-memory database.")
    }
    fn setup_playlist(db: & DBController) -> i64 {
        let playlist_name = "My favorite Songs";
        
        // The function create_playlist returns a variable Result<i64> with the playlist id 
        let result = db.create_playlist(playlist_name);

        assert!(result.is_ok(), "Failed to create the playlist in the database.");

        result.unwrap()
    }
    // --- TEST 1: Database Creation ---
    #[test]
    fn test_in_memory_database_creation() {
        // 1. Instantiate the controller in RAM memory
        let db = setup_memory_db();

        // 2. Check if the tables actually exist by querying SQLite
        // sqlite_master is an internal table that stores the database schema
        let query = "SELECT count(name) FROM sqlite_master WHERE type='table' AND name='Playlist';";
        
        let mut table_exists = false;
        
        // Execute the query
        db.connection.iterate(query, |pairs| {
            for &(_column, value) in pairs.iter() {
                // If the returned value is "1", the table exists
                if value == Some("1") {
                    table_exists = true;
                }
            }
            true
        }).expect("Error querying sqlite_master.");

        // 4. Assert that the table must exist. If false, the test fails and panics.
        assert!(table_exists, "The 'Playlist' table was not created during initialization.");
    }

    // --- TEST 2: Create Playlist ---
    #[test]
    fn test_create_playlist() {
        let db = setup_memory_db();

        let playlist_name = "My Favorite Songs";

        let result = db.create_playlist(playlist_name);

        assert!(result.is_ok(), "Failed to create the playlist in the database.");

        let new_id = result.unwrap();
        assert_eq!(new_id, 1, "The first created playlist should have ID 1.");
    }

    // --- TEST 3: Delete Playlist --- 
    #[test]
    fn test_delete_playlist() {
        let db = setup_memory_db();

        let id_playlist: i64 = setup_playlist(&db);

        let result_deletion = db.delete_playlist(id_playlist);

        assert!(result_deletion.is_ok(), "Failed to delete the playlist in the database.");
    }

    // --- TEST 4: Add Song to a Playlist --- 
    #[test]
    fn test_add_song_to_playlist() {
        let db = setup_memory_db();

        let id_playlist: i64 = setup_playlist(&db);

        let song = Song::new(
            None, 
            "SongFav".to_string(), 
            60, 
            "".to_string(), 
            SongOrigin::Local(LocalSongData {file_direction: "somewhere".to_string()}),
        );
        
        assert!(db.add_song_to_playlist(id_playlist, song).is_ok(), "Failed to add a song to a playlist in the database.");
    }

    // --- TEST 5: Get Playlists ---
    #[test]
    fn test_get_playlists() {
        let db = setup_memory_db();

        // 1. Create an initial playlist
        let playlist_name = "Rock Classics";
        let playlist_id = db.create_playlist(playlist_name)
            .expect("Failed to create the initial playlist");

        // 2. Prepare a couple of songs of different origins
        let song1 = Song::new(
            None,
            "Bohemian Rhapsody".to_string(),
            354, // Seconds
            "".to_string(),
            SongOrigin::Local(LocalSongData { file_direction: "/music/queen/bohemian.mp3".to_string() }),
        );
        
        let song2 = Song::new(
            None,
            "Hotel California".to_string(),
            390, // Seconds
            "https://spotify.com/img/...".to_string(),
            SongOrigin::Spotify(SpotifySongData { spotify_id: "spotify:track:40riOy7x9W7GXjyhghGhw9".to_string() }),
        );

        // 3. Insert them into the database and link them to the playlist
        db.add_song_to_playlist(playlist_id, song1).expect("Failed to add song1");
        db.add_song_to_playlist(playlist_id, song2).expect("Failed to add song2");

        // 4. Execute the function to be tested
        let fetch_result = db.get_playlists();
        assert!(fetch_result.is_ok(), "Failed to execute the get_playlists query");

        let playlists = fetch_result.unwrap();

        // 5. Verify that it fetched the correct playlist
        assert_eq!(playlists.len(), 1, "There should be exactly 1 playlist in the database");
        
        let fetched_playlist = &playlists[0];
        assert_eq!(fetched_playlist.playlist_name(), playlist_name, "The fetched playlist name does not match");

        // 6. Check the integrity of the internal data (The songs)
        // 354s + 390s = 744 total seconds.
        // If get_duration yields 744, it means the INNER JOIN and Song mapping worked perfectly.
        assert_eq!(
            fetched_playlist.get_duration(), 
            744, 
            "The total duration does not match. The songs were not correctly hydrated when fetching the playlist."
        );
    }
}
