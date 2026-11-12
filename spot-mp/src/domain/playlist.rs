use crate::domain::song::Song;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalPlaylistData {
    pub id_playlist: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpotifyPlaylistData {
    pub id_playlist: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlaylistOrigin {
    Local(LocalPlaylistData),
    Spotify(SpotifyPlaylistData),
}

#[derive(Clone, Debug)]
pub enum SpotifySearchResponse {
    Song(Song),
    Playlist(Playlist),
    Album(Playlist),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Playlist {
    songs: Vec<Song>,
    is_modifiable: bool,
    playlist_name: String,
    id_playlist: PlaylistOrigin,
}

impl Playlist {
    pub fn new(
        songs: Vec<Song>,
        is_modifiable: bool,
        playlist_name: String,
        id_playlist: PlaylistOrigin,
    ) -> Self {
        Self {
            songs,
            is_modifiable,
            playlist_name,
            id_playlist,
        }
    }

    pub fn playlist_name(&self) -> &str {
        &self.playlist_name
    }

    pub fn is_modifiable(&self) -> bool {
        self.is_modifiable
    }

    pub fn songs(&self) -> &[Song] {
        &self.songs
    }

    pub fn origin(&self) -> &PlaylistOrigin {
        &self.id_playlist
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
