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
    AddedSongTo(Song, String), // song -> Playlist Id
    ErasedSongFrom(i64, String), // song -> Playlist Id
}

pub enum SpotifyAction {
    LoggedIn,
    LoggedOut,
    UserIsLoggedOut,
    ExpiredToken(FunctionToRepeat),
    FetchedPlaylists(Vec<Playlist>),
    AddedSongTo(Song, String), // Playlist Id
    RemovedSongFrom(Song, String), // Playlist Id
    PlaylistCreatedWith(Playlist, Option<Song>), 
    QueryResult(Vec<Song>),
}

pub enum FunctionToRepeat {
    FetchedPlaylists,
    RemoveSongTo(Song, String), // Playlist Id
    AddSongTo(Song, String), // Playlist Id
    CreatePlaylistWith(String, bool, Option<Song>), // Playlist Name | Public | Optional Song to add
    SearchQuery(String), // Query
}

pub enum MusicPlayerAction {
    SongFinished,
    StartPlayingSong(Song), 
    LibrespotInitialized(LibrespotData),
}


