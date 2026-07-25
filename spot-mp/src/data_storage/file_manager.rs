use std::{env, fs, path::Path};

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
        
        let song_fd = env::var("SPOT_MP_SONG_FOLDER").unwrap_or_else(|_| "~/Music/".to_string());
        let cache_fd = env::var("SPOT_MP_CACHE_FOLDER").unwrap_or_else(|_| "~/.spot-mp/cache/".to_string());
        
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

    pub fn check_file_existance(&self, file_name: String) -> bool{
 
        let folder_file_name = self.song_folder() + file_name.as_str();
        
        let file_path = Path::new(&folder_file_name);
        
        file_path.exists()
    }
}
