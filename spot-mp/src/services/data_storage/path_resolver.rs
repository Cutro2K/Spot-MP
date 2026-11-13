use std::{
    fs,
    path::{Path, PathBuf},
};

use directories::{ProjectDirs, UserDirs};

pub struct PathResolver;

impl PathResolver {
    pub fn resolve_config_folder() -> PathBuf {
        if let Some(user_dirs) = UserDirs::new() {
            let spot_mp_folder = if let Some(project_dir) = ProjectDirs::from("", "", "Spot-MP") {
                project_dir.config_dir().to_path_buf()
            } else {
                user_dirs.home_dir().join(Path::new("Spot-MP/"))
            };

            Self::ensure_dir_exists(&spot_mp_folder, "Spot-MP");
            return spot_mp_folder;
        }
        panic!("Spot-mp couldn't start UserDirs!");
    }

    pub fn resolve_default_paths(config_folder: &Path) -> (PathBuf, PathBuf) {
        let user_dirs = UserDirs::new();
        let music_folder = user_dirs
            .as_ref()
            .and_then(|u| u.audio_dir().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| config_folder.join("Music"));

        let cache_folder = if let Some(project_dir) = ProjectDirs::from("", "", "Spot-MP") {
            project_dir.cache_dir().to_path_buf()
        } else {
            config_folder.join("Cache")
        };

        (music_folder, cache_folder)
    }

    pub fn ensure_dir_exists(dir: &Path, label: &str) {
        if !dir.exists() {
            fs::create_dir_all(dir).unwrap_or_else(|_| {
                panic!(
                    "Failed to try to create {} folder: {}",
                    label,
                    dir.display()
                )
            });
        }
    }
}
