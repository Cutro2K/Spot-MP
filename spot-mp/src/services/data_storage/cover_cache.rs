use std::{
    fs, io,
    path::{Path, PathBuf},
};

use audiotags::MimeType;
use log::error;

const DEFAULT_BROKEN_IMAGE: &[u8] = include_bytes!("../../../assets/images/broken-image.png");

pub struct CoverArtCache {
    cache_folder: PathBuf,
}

impl CoverArtCache {
    pub fn new(cache_folder: PathBuf) -> Self {
        Self { cache_folder }
    }

    pub fn cache_folder(&self) -> PathBuf {
        self.cache_folder.clone()
    }

    pub fn default_cover() -> &'static [u8] {
        DEFAULT_BROKEN_IMAGE
    }

    pub fn mime_to_extension(mime: MimeType) -> &'static str {
        match mime {
            MimeType::Jpeg => ".jpg",
            MimeType::Png => ".png",
            MimeType::Gif => ".gif",
            MimeType::Bmp => ".bmp",
            MimeType::Tiff => ".tiff",
        }
    }

    pub fn check_cache_existence(&self, file_name: &str) -> bool {
        self.cache_folder.join(file_name).exists()
    }

    pub fn save_cover(&self, song_title: &str, extension: &str, cover_bytes: &[u8]) -> PathBuf {
        let safe_title = song_title.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        let song_cover_name = format!("{}{}", safe_title, extension);
        let cover_cache_path = self.cache_folder.join(&song_cover_name);

        if !self.check_cache_existence(&song_cover_name) {
            fs::write(&cover_cache_path, cover_bytes).unwrap_or_else(|e| {
                error!(
                    "The cover image didn't save correctly for {} | error: {}",
                    song_cover_name, e
                )
            });
        }

        cover_cache_path
    }

    pub fn get_img(img_path: &str) -> io::Result<Vec<u8>> {
        if img_path.is_empty() {
            return Ok(DEFAULT_BROKEN_IMAGE.to_vec());
        }
        fs::read(Path::new(img_path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_save_and_get_cover_in_cache() {
        let temp_dir = env::temp_dir().join(format!("spot-mp-cache-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let cache = CoverArtCache::new(temp_dir.clone());
        let saved_path = cache.save_cover("Song: With / Invalid * Chars?", ".png", b"fake-img");

        assert!(saved_path.exists());
        assert!(cache.check_cache_existence("Song_ With _ Invalid _ Chars_.png"));

        let loaded = CoverArtCache::get_img(saved_path.to_str().unwrap()).unwrap();
        assert_eq!(loaded, b"fake-img");

        let fallback = CoverArtCache::get_img("").unwrap();
        assert_eq!(fallback, CoverArtCache::default_cover());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
