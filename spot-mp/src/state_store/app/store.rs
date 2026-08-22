use crate::state_store::app::action::Action;

// This store have the entire app state
#[allow(dead_code)]
pub struct Store {
    local_page: LocalPageStore,
    spotify_page: SpotifyPageStore,
    playlist_page: PlaylistStore,
    music_player: MusicPlayerStore,
}

impl Store {
    pub fn update(&mut self, _action: Action) {
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
