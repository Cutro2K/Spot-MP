use crate::domain::song::Song;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Album {
    id_album: String,
    album_name: String,
    artist: String,
    release_year: Option<i32>,
    img_path: String,
    songs: Vec<Song>,
}

impl Album {
    pub fn new(
        id_album: String,
        album_name: String,
        artist: String,
        release_year: Option<i32>,
        img_path: String,
        songs: Vec<Song>,
    ) -> Self {
        Self {
            id_album,
            album_name,
            artist,
            release_year,
            img_path,
            songs,
        }
    }

    pub fn id_album(&self) -> &str {
        &self.id_album
    }

    pub fn album_name(&self) -> &str {
        &self.album_name
    }

    pub fn artist(&self) -> &str {
        &self.artist
    }

    pub fn release_year(&self) -> Option<i32> {
        self.release_year
    }

    pub fn img_path(&self) -> &str {
        &self.img_path
    }

    pub fn songs(&self) -> &[Song] {
        &self.songs
    }

    pub fn get_duration(&self) -> i64 {
        self.songs.iter().map(|s| s.duration()).sum()
    }

    pub fn add_song(&mut self, song: Song) {
        self.songs.push(song);
    }
}