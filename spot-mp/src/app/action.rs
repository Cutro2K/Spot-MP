use crate::domain::{auth_models::LibrespotData, song::Song};

pub enum Action {
    Local(LocalAction),
    Spotify(SpotifyAction),
    MusicPlayer(MusicPlayerAction),
    NoOp,
    UnexpectedErrorHappened(String), // Error description
}

pub enum LocalAction {

}

pub enum SpotifyAction {

}

pub enum MusicPlayerAction {
    SongFinished,
    StartPlayingSong(Song), 
    LibrespotInitialized(LibrespotData),
}
