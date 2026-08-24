use crate::domain::{auth_models::LibrespotData, playlist::Playlist, song::Song};

pub enum Action {
    Local(LocalAction),
    Spotify(SpotifyAction),
    MusicPlayer(MusicPlayerAction),
    NoOp,
    UnexpectedErrorHappened(String), // Error description
}

pub enum LocalAction {
    FetchedPlaylists(Vec<Playlist>),
    FetchedSongs(Vec<Song>),
    CreatedPlaylist(Playlist),
    DeletedPlaylist(String), // Playlist id
    AddedSongTo(Song, String), // song -> PlaylistId
    ErasedSongFrom(i64, String), // song -> PlaylistId
}

pub enum SpotifyAction {

}

pub enum MusicPlayerAction {
    SongFinished,
    StartPlayingSong(Song), 
    LibrespotInitialized(LibrespotData),
}


