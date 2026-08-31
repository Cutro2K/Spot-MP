use crate::{domain::{auth_models::LibrespotData, playlist::Playlist, song::Song}, services::spotify_api::spotify_api_client::SpotifySearchResponse};

pub enum Action {
    Local(LocalAction),
    Spotify(SpotifyAction),
    MusicPlayer(MusicPlayerAction),
    NoOp,
    UnexpectedErrorHappened(String), // Error description
}

pub enum LocalAction {
    FetchPlaylists,
    FetchSongs,
    CreatePlaylist(String), // Playlist id
    DeletePlaylist(i64),
    AddSongTo(Song, i64), // song -> Playlist id
    EraseSongFrom(i64, i64), // song_id -> Playlist id
                            
    // Response Actions | controllers -> Dispatcher
    FetchedPlaylists(Vec<Playlist>),
    FetchedSongs(Vec<Song>),
    CreatedPlaylist(Playlist),
    DeletedPlaylist(i64), // Playlist id
    AddedSongTo(Song, i64), // song -> Playlist Id
    ErasedSongFrom(i64, i64), // song_id -> Playlist Id
}

pub enum SpotifyAction {

    // Response Actions | controllers -> Dispatcher
    LoggedIn,
    LoggedOut,
    UserIsLoggedOut,
    ExpiredToken(FunctionToRepeat),
    FetchedPlaylists(Vec<Playlist>),
    AddedSongTo(Song, String), // Playlist Id
    RemovedSongFrom(Song, String), // Playlist Id
    PlaylistCreatedWith(Playlist, Option<Song>), 
    QueryResult(Vec<SpotifySearchResponse>),
}

pub enum FunctionToRepeat {

    // Response Actions | controllers -> Dispatcher
    FetchedPlaylists,
    RemoveSongTo(Song, String), // Playlist Id
    AddSongTo(Song, String), // Playlist Id
    CreatePlaylistWith(String, bool, Option<Song>), // Playlist Name | Public | Optional Song to add
    SearchQuery(String), // Query
}

pub enum MusicPlayerAction {
    
    // Response Actions | controllers -> Dispatcher
    SongFinished,
    StartPlayingSong(Song), 
    LibrespotInitialized(LibrespotData),
}


