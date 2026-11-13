use std::{
    fs, io,
    path::{Path, PathBuf},
};

use log::error;
use serde::{Deserialize, Serialize};

use crate::services::data_storage::path_resolver::PathResolver;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub music_folder: PathBuf,
    pub cache_folder: PathBuf,
}

pub struct ConfigStorage {
    config_folder: PathBuf,
    config: AppConfig,
}

impl Default for ConfigStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigStorage {
    pub fn new() -> Self {
        let config_folder = PathResolver::resolve_config_folder();
        Self::with_config_folder(config_folder)
    }

    pub fn with_config_folder(config_folder: PathBuf) -> Self {
        PathResolver::ensure_dir_exists(&config_folder, "Spot-MP");

        let config_file_path = config_folder.join("config.json");

        let (music_folder, cache_folder) = if config_file_path.exists() {
            // Second start onwards: read from config.json
            match fs::read_to_string(&config_file_path) {
                Ok(content) => match serde_json::from_str::<AppConfig>(&content) {
                    Ok(loaded_config) => (loaded_config.music_folder, loaded_config.cache_folder),
                    Err(err) => {
                        error!(
                            "Failed to parse config.json at {}: {}. Generating default config.",
                            config_file_path.display(),
                            err
                        );
                        Self::create_and_persist_defaults(&config_folder, &config_file_path)
                    }
                },
                Err(err) => {
                    error!(
                        "Failed to read config.json at {}: {}. Generating default config.",
                        config_file_path.display(),
                        err
                    );
                    Self::create_and_persist_defaults(&config_folder, &config_file_path)
                }
            }
        } else {
            // First start: resolve default paths and save config.json
            Self::create_and_persist_defaults(&config_folder, &config_file_path)
        };

        PathResolver::ensure_dir_exists(&music_folder, "Music");
        PathResolver::ensure_dir_exists(&cache_folder, "Cache");

        Self {
            config_folder,
            config: AppConfig {
                music_folder,
                cache_folder,
            },
        }
    }

    fn create_and_persist_defaults(
        config_folder: &Path,
        config_file_path: &Path,
    ) -> (PathBuf, PathBuf) {
        let (def_music, def_cache) = PathResolver::resolve_default_paths(config_folder);
        let config = AppConfig {
            music_folder: def_music.clone(),
            cache_folder: def_cache.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&config) {
            if let Err(e) = fs::write(config_file_path, json) {
                error!(
                    "Failed to save default config.json at {}: {}",
                    config_file_path.display(),
                    e
                );
            }
        }
        (def_music, def_cache)
    }

    pub fn config_folder(&self) -> PathBuf {
        self.config_folder.clone()
    }

    pub fn music_folder(&self) -> PathBuf {
        self.config.music_folder.clone()
    }

    pub fn cache_folder(&self) -> PathBuf {
        self.config.cache_folder.clone()
    }

    pub fn config_file_path(&self) -> PathBuf {
        self.config_folder.join("config.json")
    }

    pub fn load_config(&self) -> io::Result<AppConfig> {
        let content = fs::read_to_string(self.config_file_path())?;
        serde_json::from_str(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn save_config(&self) -> io::Result<()> {
        let json = serde_json::to_string_pretty(&self.config)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(self.config_file_path(), json)
    }

    pub fn set_music_folder(&mut self, new_folder: PathBuf) -> io::Result<()> {
        if !new_folder.exists() {
            fs::create_dir_all(&new_folder)?;
        }
        self.config.music_folder = new_folder;
        self.save_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn create_temp_test_dir(name: &str) -> PathBuf {
        let temp_dir =
            env::temp_dir().join(format!("spot-mp-test-{}-{}", name, uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).expect("Failed to create temporary test directory");
        temp_dir
    }

    #[test]
    fn test_first_start_creates_config_json() {
        let temp_dir = create_temp_test_dir("first_start");
        let config_file = temp_dir.join("config.json");
        assert!(!config_file.exists());

        let cs = ConfigStorage::with_config_folder(temp_dir.clone());
        assert!(config_file.exists());

        let loaded = cs.load_config().expect("Should load valid AppConfig");
        assert_eq!(loaded.music_folder, cs.music_folder());
        assert_eq!(loaded.cache_folder, cs.cache_folder());
        assert!(loaded.music_folder.exists());
        assert!(loaded.cache_folder.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_second_start_reads_existing_config_json() {
        let temp_dir = create_temp_test_dir("second_start");
        let custom_music = temp_dir.join("MyCustomMusic");
        let custom_cache = temp_dir.join("MyCustomCache");

        let custom_config = AppConfig {
            music_folder: custom_music.clone(),
            cache_folder: custom_cache.clone(),
        };

        let config_file = temp_dir.join("config.json");
        let json = serde_json::to_string_pretty(&custom_config).unwrap();
        fs::write(&config_file, json).unwrap();

        // Second start should read config.json instead of defaults
        let cs = ConfigStorage::with_config_folder(temp_dir.clone());
        assert_eq!(cs.music_folder(), custom_music);
        assert_eq!(cs.cache_folder(), custom_cache);
        assert!(custom_music.exists());
        assert!(custom_cache.exists());

        let loaded = cs.load_config().expect("Should load valid AppConfig");
        assert_eq!(loaded, custom_config);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_set_music_folder_updates_config_json() {
        let temp_dir = create_temp_test_dir("set_music");
        let mut cs = ConfigStorage::with_config_folder(temp_dir.clone());

        let updated_music = temp_dir.join("UpdatedMusicFolder");
        cs.set_music_folder(updated_music.clone())
            .expect("Should update music folder");

        assert_eq!(cs.music_folder(), updated_music);
        assert!(updated_music.exists());

        let loaded = cs.load_config().expect("Should load valid AppConfig");
        assert_eq!(loaded.music_folder, updated_music);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
