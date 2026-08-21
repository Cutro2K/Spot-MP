use crate::app::action::Action;

// This store have the entire app state 
pub struct Store {
    local_page: LocalPageStore,
    spotify_page: SpotifyPageStore,
    playlist_page: PlaylistStore,
    music_player: MusicPlayerStore,
}

impl Store {
    pub fn update(&mut self, action: Action) {
       match action {
           _ => {},
       }
    }
}

// This struct have the specific page state: 

pub struct LocalPageStore {

}

pub struct SpotifyPageStore {

}

pub struct PlaylistStore { 
    
}

pub struct MusicPlayerStore {

}
