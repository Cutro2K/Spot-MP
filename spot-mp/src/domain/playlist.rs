use crate::domain::song::Song;

pub struct LocalPlaylistData {
    pub id_playlist: i64,
}

pub struct SpotifyPlaylistData {
    pub id_playlist: String,
}

pub enum PlaylistOrigin {
    Local(LocalPlaylistData),
    Spotify(SpotifyPlaylistData),
}

pub struct Playlist {
    songs: Vec<Song>,
    is_modifiable: bool,
    playlist_name: String,
    id_playlist: PlaylistOrigin,
}

impl Playlist {
    pub fn new(_songs: Vec<Song>, _is_modifiable: bool, _playlist_name: String, _id_playlist: PlaylistOrigin) -> Self {
        Self {
            songs: _songs,
            is_modifiable: _is_modifiable,
            playlist_name: _playlist_name,
            id_playlist: _id_playlist,
        }
    }

    pub fn playlist_name(&self) -> String {
        self.playlist_name.clone()
    }

    pub fn is_modifiable(&self) -> bool {
        self.is_modifiable
    }
    
    pub fn id_playlist(&self) -> String {
        match &self.id_playlist {
            PlaylistOrigin::Local(playlist_data) => playlist_data.id_playlist.to_string(),
            PlaylistOrigin::Spotify(playlist_data) => playlist_data.id_playlist.clone(),
        }
    }

    pub fn get_duration(&self) -> i64 {
        self.songs.iter().map(|s| s.duration()).sum()
    }

    pub fn add_song(&mut self, song: Song) {
        self.songs.push(song);
   }
}

