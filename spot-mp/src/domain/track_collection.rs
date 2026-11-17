use crate::domain::{album::Album, playlist::Playlist, song::Song};

pub trait TrackCollection {
    fn id(&self) -> String;
    fn name(&self) -> &str;
    fn songs(&self) -> &[Song];
    fn is_modifiable(&self) -> bool;
    fn img_url(&self) -> Option<&str>;
    fn get_duration(&self) -> i64 {
        self.songs().iter().map(|s| s.duration()).sum()
    }
}

impl TrackCollection for Playlist {
    fn id(&self) -> String {
        self.id_playlist()
    }

    fn name(&self) -> &str {
        self.playlist_name()
    }

    fn songs(&self) -> &[Song] {
        self.songs()
    }

    fn is_modifiable(&self) -> bool {
        self.is_modifiable()
    }

    fn img_url(&self) -> Option<&str> {
        self.get_img()
    }
}

impl TrackCollection for Album {
    fn id(&self) -> String {
        self.id_album().to_string()
    }

    fn name(&self) -> &str {
        self.album_name()
    }

    fn songs(&self) -> &[Song] {
        self.songs()
    }

    fn is_modifiable(&self) -> bool {
        false
    }

    fn img_url(&self) -> Option<&str> {
        if self.img_path().is_empty() {
            None
        } else {
            Some(self.img_path())
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Collection {
    Playlist(Playlist),
    Album(Album),
}

impl TrackCollection for Collection {
    fn id(&self) -> String {
        match self {
            Collection::Playlist(p) => p.id(),
            Collection::Album(a) => a.id(),
        }
    }

    fn name(&self) -> &str {
        match self {
            Collection::Playlist(p) => p.name(),
            Collection::Album(a) => a.name(),
        }
    }

    fn songs(&self) -> &[Song] {
        match self {
            Collection::Playlist(p) => p.songs(),
            Collection::Album(a) => a.songs(),
        }
    }

    fn is_modifiable(&self) -> bool {
        match self {
            Collection::Playlist(p) => p.is_modifiable(),
            Collection::Album(a) => a.is_modifiable(),
        }
    }

    fn img_url(&self) -> Option<&str> {
        match self {
            Collection::Playlist(p) => p.img_url(),
            Collection::Album(a) => a.img_url(),
        }
    }
}