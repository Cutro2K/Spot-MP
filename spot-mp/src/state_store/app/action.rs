use crate::domain::{
    album::Album,
    auth_models::LibrespotData,
    playback_flow::PlaybackFlow,
    playlist::{Playlist, SpotifySearchResponse},
    song::Song,
};

#[derive(Debug, Clone)]
pub enum Action {
    Local(LocalAction),
    Spotify(SpotifyAction),
    MusicPlayer(MusicPlayerAction),
    NoOp,
    UnexpectedErrorHappened(String), // Error description
}

#[derive(Debug, Clone)]
pub enum LocalAction {
    // Task Actions | Dispatcher -> ControllerHub
    FetchPlaylists,
    FetchSongs,
    CreatePlaylist(String),  // Playlist name
    DeletePlaylist(i64),     // Playlist id
    AddSongTo(Song, i64),    // song -> Playlist id
    EraseSongFrom(i64, i64), // song_id -> Playlist id

    // Response Actions | Controllers -> Dispatcher
    FetchedPlaylists(Vec<Playlist>),
    FetchedSongs(Vec<Song>),
    CreatedPlaylist(Playlist),
    DeletedPlaylist(i64),     // Playlist id
    AddedSongTo(Song, i64),   // song -> Playlist Id
    ErasedSongFrom(i64, i64), // song_id -> Playlist Id
}

#[derive(Debug, Clone)]
pub enum SpotifyAction {
    // Task Actions | Dispatcher -> ControllerHub
    LoggIn(String), // Spotify ClientId
    LoggOut,
    FetchPlaylists,
    FetchAlbums,
    SearchQuery(String),                         // String Query
    AddSongTo(String, Song),                     // PlaylistId, Song
    EraseSongFrom(String, Song),                 // PlaylistId, Song
    CreatePlaylist(String, bool, Option<Song>),  // PlaylistName, Public Boolean, Optional initial song
    DeletePlaylist(String),                      // PlaylistId (Unfollow)
    DeleteAlbum(String),                         // AlbumId (Unfollow)

    // Response Actions | ControllerHub -> Dispatcher
    LoggedIn,
    LoggedOut,
    UserIsLoggedOut,
    SessionExpired,
    FetchedPlaylists(Vec<Playlist>),
    FetchedAlbums(Vec<Album>),
    AddedSongTo(Song, String),     // Playlist Id
    RemovedSongFrom(Song, String), // Playlist Id
    PlaylistCreatedWith(Playlist, Option<Song>),
    DeletedPlaylist(String),       // Playlist Id
    DeletedAlbum(String),          // Album Id
    QueryResult(Vec<SpotifySearchResponse>),
}

#[derive(Debug, Clone)]
pub enum MusicPlayerAction {
    // Task Actions | Dispatcher -> ControllerHub
    PlaySong(Song),
    PlayPlaylist(Vec<Song>),
    AddToQueue(Song),
    EraseFromQueue(String),
    ClearQueue,
    TogglePause,
    Seek(u32),
    SkipToNext,
    SkipToPrevious,
    ChangeFlow(PlaybackFlow),

    // Response Actions | ControllerHub -> Dispatcher
    SongFinished,
    StartPlayingSong(Song),
    LibrespotInitialized(LibrespotData),
}
