use std::fs::File;
use std::{fs, io, path::Path, path::PathBuf};
use audiotags::{MimeType, Tag};
use directories::{ProjectDirs, UserDirs};
use log::error;
use simplelog::{Config, LevelFilter, WriteLogger};

use crate::domain::song::{LocalSongData, Song, SongOrigin};
use crate::domain::spot_mp_errors::SpotMpError;

pub struct FileManager {
    config_folder: PathBuf,
    music_folder: PathBuf,
    cache_folder: PathBuf,
}

impl Default for FileManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FileManager {
    pub fn new() -> Self {
        if let Some(user_dirs) = UserDirs::new() {
            let home_dir = user_dirs.home_dir();

            if let Some(project_dir) = ProjectDirs::from("", "", "Spot-MP") {
                let spot_mp_folder = project_dir.config_dir().to_path_buf();
                if !spot_mp_folder.exists() {
                    fs::create_dir_all(&spot_mp_folder).unwrap_or_else(|_| {
                        panic!(
                            "Failed to try to create Spot-MP folder: {}",
                            spot_mp_folder.display()
                        )
                    });
                }
                let config_folder = spot_mp_folder.clone();

                let music_folder = match user_dirs.audio_dir() {
                    Some(dir) => dir.to_path_buf(),
                    None => spot_mp_folder.clone().join(Path::new("Music/")),
                };
                if !music_folder.exists() {
                    fs::create_dir_all(&music_folder).unwrap_or_else(|_| {
                        panic!(
                            "Failed to try to create Music folder: {}",
                            music_folder.display()
                        )
                    });
                }

                let cache_folder = project_dir.cache_dir().to_path_buf();
                if !cache_folder.exists() {
                    fs::create_dir_all(&cache_folder).unwrap_or_else(|_| {
                        panic!(
                            "Failed to try to create Cache folder: {}",
                            cache_folder.display()
                        )
                    });
                }

                Self {
                    config_folder,
                    music_folder,
                    cache_folder,
                }
            } else {
                let spot_mp_folder = home_dir.join(Path::new("Spot-MP/"));
                if !spot_mp_folder.exists() {
                    fs::create_dir_all(&spot_mp_folder).unwrap_or_else(|_| {
                        panic!(
                            "Failed to try to create Spot-MP folder: {}",
                            spot_mp_folder.display()
                        )
                    });
                }
                let config_folder = spot_mp_folder.clone();

                let music_folder = match user_dirs.audio_dir() {
                    Some(dir) => dir.to_path_buf(),
                    None => spot_mp_folder.clone().join(Path::new("Music/")),
                };
                if !music_folder.exists() {
                    fs::create_dir_all(&music_folder).unwrap_or_else(|_| {
                        panic!(
                            "Failed to try to create Music folder: {}",
                            music_folder.display()
                        )
                    });
                }

                let cache_folder = spot_mp_folder.clone().join(Path::new("Cache/"));
                if !cache_folder.exists() {
                    fs::create_dir_all(&cache_folder).unwrap_or_else(|_| {
                        panic!(
                            "Failed to try to create Cache folder: {}",
                            cache_folder.display()
                        )
                    });
                }

                Self {
                    config_folder,
                    music_folder,
                    cache_folder,
                }
            }
        } else {
            panic!("Spot-mp couldn't start UserDirs class for file manager!");
        }
    }

    pub fn song_folder(&self) -> PathBuf {
        self.music_folder.clone()
    }

    pub fn cache_folder(&self) -> PathBuf {
        self.cache_folder.clone()
    }

    pub fn config_folder(&self) -> PathBuf {
        self.config_folder.clone()
    }

    pub fn check_file_existence(&self, file_name: &str) -> bool {
        let candidate = Path::new(file_name);
        if candidate.is_absolute() {
            candidate.exists()
        } else {
            self.music_folder.join(file_name).exists()
        }
    }

    fn check_cache_existence(&self, file_name: &str) -> bool {
        self.cache_folder().join(file_name).exists()
    }

    fn mime_to_extension(mime: MimeType) -> &'static str {
        match mime {
            MimeType::Jpeg => ".jpg",
            MimeType::Png => ".png",
            MimeType::Gif => ".gif",
            MimeType::Bmp => ".bmp",
            MimeType::Tiff => ".tiff",
        }
    }

    pub fn get_local_songs(&self) -> Result<Vec<Song>, SpotMpError> {
        let mut songs: Vec<Song> = Vec::new();

        for song in fs::read_dir(self.song_folder())? {
            let song_file = song?;
            let file_path = song_file.path();

            // Skip subdirectories and non-regular files
            if !file_path.is_file() {
                continue;
            }

            // Default values:
            let mut song_title = song_file.file_name().display().to_string();
            let mut song_artist = String::from("Unknown Artist");
            let mut song_album = String::from("Unknown Album");
            let mut song_year: Option<i32> = None;
            let mut song_duration = 0;
            let mut song_cover: &[u8] = include_bytes!("../../../assets/images/broken-image.png");
            let mut cover_extension = ".png";

            let song_tag;
            match Tag::new().read_from_path(&file_path) {
                Ok(tag) => {
                    song_tag = tag;
                    if let Some(title) = song_tag.title() {
                        song_title = title.to_string();
                    }
                    if let Some(artist) = song_tag.artist() {
                        song_artist = artist.to_string();
                    }
                    if let Some(album) = song_tag.album_title() {
                        song_album = album.to_string();
                    }
                    song_year = song_tag.year();
                    if let Some(duration) = song_tag.duration() {
                        song_duration = duration as i64;
                    }

                    if let Some(picture) = song_tag.album_cover() {
                        cover_extension = Self::mime_to_extension(picture.mime_type);
                        song_cover = picture.data;
                    }
                }
                Err(audiotags::Error::UnknownFileExtension(_)) => {
                    continue;
                }
                _ => {}
            }

            // Sanitize file name for cover cache
            let safe_title = song_title.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
            let song_cover_name = format!("{}{}", safe_title, cover_extension);
            let cover_cache_path = self.cache_folder().join(&song_cover_name);

            if !self.check_cache_existence(&song_cover_name) {
                fs::write(&cover_cache_path, song_cover).unwrap_or_else(|e| {
                    error!(
                        "The cover image didn't save correctly for {} | error: {}",
                        song_cover_name, e
                    )
                });
            }

            let new_song = Song::with_metadata(
                None,
                song_title,
                song_artist,
                song_album,
                song_year,
                song_duration,
                cover_cache_path.display().to_string(),
                SongOrigin::Local(LocalSongData {
                    file_direction: file_path.display().to_string(),
                }),
            );

            songs.push(new_song);
        }

        Ok(songs)
    }

    pub fn get_img(img_path: &str) -> io::Result<Vec<u8>> {
        if img_path.is_empty() {
            return Ok(include_bytes!("../../../assets/images/broken-image.png").to_vec());
        }
        fs::read(Path::new(img_path))
    }

    fn resolve_config_folder() -> PathBuf {
        if let Some(user_dirs) = UserDirs::new() {
            if let Some(project_dir) = ProjectDirs::from("", "", "Spot-MP") {
                let spot_mp_folder = project_dir.config_dir().to_path_buf();
                if !spot_mp_folder.exists() {
                    fs::create_dir_all(&spot_mp_folder)
                        .unwrap_or_else(|_| panic!("Failed to try to create Spot-MP folder"));
                }
                return spot_mp_folder;
            } else {
                let home_dir = user_dirs.home_dir();
                let spot_mp_folder = home_dir.join(Path::new("Spot-MP/"));
                if !spot_mp_folder.exists() {
                    fs::create_dir_all(&spot_mp_folder)
                        .unwrap_or_else(|_| panic!("Failed to try to create Spot-MP folder"));
                }
                return spot_mp_folder;
            }
        }
        panic!("Spot-mp couldn't start UserDirs!");
    }

    pub fn init_logger() {
        let config_folder = Self::resolve_config_folder();
        let log_path = config_folder.join("spot-mp.log");

        WriteLogger::init(
            LevelFilter::Info,
            Config::default(),
            File::create(log_path).expect("Couldn't create the log file in config folder"),
        )
        .unwrap();
    }
}
