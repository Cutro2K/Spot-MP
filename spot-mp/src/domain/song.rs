#[derive(Clone, Debug)]
pub struct LocalSongData {
    pub file_direction: String,
}

#[derive(Clone, Debug)]
pub struct SpotifySongData {
    pub spotify_id: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum SongOrigin {
    Local(LocalSongData),
    Spotify(SpotifySongData),
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Song {
    id_song: Option<i64>,
    song_name: String,
    duration: i64,
    img_path: String,
    song: SongOrigin,
}

impl Song {
    pub fn new(id_song: Option<i64>, song_name: String, duration: i64, img_path: String, song: SongOrigin) -> Self {
        Self {
            id_song,
            song_name,
            duration,
            img_path,
            song,
        }
    }
    pub fn get_song_name(&self) -> String {
        self.song_name.clone()
    }
    pub fn id_song(&self) -> i64 {
        self.id_song.unwrap()
    }
    pub fn set_id_song(&mut self, id: i64) {
        self.id_song = Some(id);
    }
    pub fn song_name(&self) -> String {
        self.song_name.clone()
    }
    pub fn duration(&self) -> i64 {
        self.duration
    }
    pub fn song(&self) -> SongOrigin {
        self.song.clone()
    }
    pub fn img_path(&self) -> String {
        self.img_path.clone()
    }

    pub fn is_local(&self) -> bool {
        match self.song {
            SongOrigin::Local(_) => true,
            SongOrigin::Spotify(_) => false,
        }
    }
}
