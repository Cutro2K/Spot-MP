use std::{
    fs,
    path::{Path, PathBuf},
};

use audiotags::Tag;

use crate::{
    domain::{
        song::{LocalSongData, Song, SongOrigin},
        spot_mp_errors::SpotMpError,
    },
    services::data_storage::cover_cache::CoverArtCache,
};

pub struct SongScanner {
    music_folder: PathBuf,
    cover_cache: CoverArtCache,
}

impl SongScanner {
    pub fn new(music_folder: PathBuf, cover_cache: CoverArtCache) -> Self {
        Self {
            music_folder,
            cover_cache,
        }
    }

    pub fn song_folder(&self) -> PathBuf {
        self.music_folder.clone()
    }

    pub fn set_music_folder(&mut self, music_folder: PathBuf) {
        self.music_folder = music_folder;
    }

    pub fn check_file_existence(&self, file_name: &str) -> bool {
        let candidate = Path::new(file_name);
        if candidate.is_absolute() {
            candidate.exists()
        } else {
            self.music_folder.join(file_name).exists()
        }
    }

    pub fn get_local_songs(&self) -> Result<Vec<Song>, SpotMpError> {
        let mut songs: Vec<Song> = Vec::new();

        for song in fs::read_dir(&self.music_folder)? {
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
            let mut song_cover: &[u8] = CoverArtCache::default_cover();
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
                        cover_extension = CoverArtCache::mime_to_extension(picture.mime_type);
                        song_cover = picture.data;
                    }
                }
                Err(audiotags::Error::UnknownFileExtension(_)) => {
                    continue;
                }
                _ => {}
            }

            let cover_cache_path =
                self.cover_cache
                    .save_cover(&song_title, cover_extension, song_cover);

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_check_file_existence() {
        let temp_dir =
            env::temp_dir().join(format!("spot-mp-scanner-test-{}", uuid::Uuid::new_v4()));
        let music_dir = temp_dir.join("music");
        let cache_dir = temp_dir.join("cache");
        fs::create_dir_all(&music_dir).unwrap();
        fs::create_dir_all(&cache_dir).unwrap();

        let dummy_song = music_dir.join("track.mp3");
        fs::write(&dummy_song, b"dummy").unwrap();

        let scanner = SongScanner::new(music_dir.clone(), CoverArtCache::new(cache_dir));

        assert!(scanner.check_file_existence("track.mp3"));
        assert!(scanner.check_file_existence(dummy_song.to_str().unwrap()));
        assert!(!scanner.check_file_existence("nonexistent.mp3"));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
