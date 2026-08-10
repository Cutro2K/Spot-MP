use std::{env, fs, io, path::Path};
use audiotags::{Tag};
use crate::domain::song::{LocalSongData, Song, SongOrigin};
use crate::domain::spot_mp_errors::SpotMpError;

#[allow(dead_code)]
pub struct FileManager {
    song_folder: String,
    cache_folder: String,
}

impl Default for FileManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FileManager {

    pub fn new() -> Self {
        let home_dir = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let song_fd = env::var("SPOT_MP_SONG_FOLDER").unwrap_or_else(|_| format!("{home_dir}/Music/"));
        let cache_fd = env::var("SPOT_MP_CACHE_FOLDER").unwrap_or_else(|_| format!("{home_dir}/.spot-mp/cache/"));
        
        // --- Checking cache folder existance and creating if not exist ---
        let cache_path = Path::new(&cache_fd);
        
        if !cache_path.exists() {
            fs::create_dir_all(cache_path).expect("Failed to try to create cache directory");
        }

        // --- Checking song folder existance and creating if not exist --- 
        
        let song_path = Path::new(&song_fd);

        if !song_path.exists() {
            fs::create_dir_all(song_path).expect("Failed to try to create song directory");
        }

        Self {
            song_folder: song_fd,
            cache_folder: cache_fd,
        }
    }

    pub fn song_folder(&self) -> String {
        self.song_folder.clone()
    }

    pub fn cache_folder(&self) -> String {
        self.cache_folder.clone()
    }

    pub fn check_file_existence(&self, file_name: String) -> bool{
        let folder_file_path = Path::new(&self.song_folder);
        let file_path = folder_file_path.join(file_name);
        file_path.exists()
    }
    
    fn check_cache_existence(&self, file_name: String) -> bool {
        let folder_cache_path = Path::new(&self.cache_folder);
        let file_path = folder_cache_path.join(file_name);
        file_path.exists()
    }

    pub fn get_local_songs(&self) -> Result<Vec<Song>, SpotMpError> {
           let mut songs: Vec<Song> = Vec::new();
           
           for song in fs::read_dir(self.song_folder())? {
                // --- Extracting file data ---
                let song_file = song?;
                let file_path = song_file.path();
                let song_tag;
                
                // Default values:
                let mut song_title = song_file.file_name().display().to_string();
                let mut song_duration = 0;
                let mut song_cover: &[u8] = include_bytes!("../../assets/images/broken-image.png");
                let mut cover_extension = ".png";
                
                // Handling non tagged files
                match Tag::new().read_from_path(&file_path) {
                    Ok(tag) => {
                        song_tag = tag;
                        // --- Extracting posible values from the file ---
                        if let Some(title) = song_tag.title() {song_title = title.to_string(); }
                        if let Some(duration) = song_tag.duration() {song_duration = duration as i64; }
                    
                        // --- Extracting picture of the image if exist ---
                        if let Some(picture) = song_tag.album_cover() {
                            cover_extension = picture.mime_type.into();
                            song_cover = picture.data;
                        }
                    },
                    Err(audiotags::Error::UnknownFileExtension(_)) => {
                        continue;
                    },
                    _ => {},
                }
                // --- Saving in cache folder if not exist ---
                let song_cover_name = song_title.to_string().replace("/", "_").replace("\\", "_") + cover_extension;
                let cover_cache_path = Path::new(&self.cache_folder()).join(&song_cover_name);

                if !self.check_cache_existence(song_cover_name.clone()) {
                    fs::write(&cover_cache_path, song_cover).unwrap_or_else(|_| println!("The cover doesn't save"));
                }
        
                // --- Creating the song ---
                let new_song = Song::new(
                    None, 
                    song_title.to_string(),
                    song_duration,
                    cover_cache_path.display().to_string(),
                    SongOrigin::Local(LocalSongData {file_direction: file_path.display().to_string()})
                );

                songs.push(new_song);
           }

           Ok(songs)
    }
    
    pub fn get_img(img_path: &str) -> io::Result<Vec<u8>> {
        fs::read(Path::new(img_path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_extract_local_songs_data() {
        let test_song_folder = "./assets/songs";
        let test_cache_folder = "./assets/cache"; 
        
        unsafe {
            env::set_var("SPOT_MP_SONG_FOLDER", test_song_folder);
            env::set_var("SPOT_MP_CACHE_FOLDER", test_cache_folder);
        }

        let file_manager = FileManager::new();

        let result = file_manager.get_local_songs();

        assert!(result.is_ok(), "The functions get_local_songs() fails");
        
        let songs = result.unwrap();

        assert!(
            !songs.is_empty(), 
            "The songs vector is empty, the file manager doesn't detect any file"
        );

        let first_song = &songs[0];
        println!("Test Completed. Song: {:?}", first_song);
    }
}
