use serde::Deserialize;

// --- Structs for responses using serde to deserialize json files ---

// --- Login ---
#[derive(Deserialize, Debug)]
pub struct SpotifyTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i32,
    pub refresh_token: Option<String>,
}

// --- API Responses ---

// -- Import playlist function response ---
#[derive(Deserialize, Debug)]
pub struct GetUserPlaylistResponse {
    pub next: Option<String>,
    pub items: Vec<PlaylistItem>,
}
#[derive(Deserialize, Debug)]
pub struct PlaylistItem {
    pub collaborative: bool,
    pub id: String, 
    pub images: Vec<Image>,
    pub owner: Owner,
    pub name: String,
}
#[derive(Deserialize, Debug)]
pub struct Owner {
    pub id: String,
}
#[derive(Deserialize, Debug)]
pub struct Image {
    pub url: String,
    pub height: Option<i32>,
    pub width: Option<i32>,
} 

// --  Get Playlist Song --

#[derive(Deserialize, Debug)]
pub struct GetPlaylistSongsResponse {
    pub next: Option<String>,
    pub items: Vec<SongItem>,
}
#[derive(Deserialize, Debug)]
pub struct SongItem {
    pub item: TrackItem,
}
#[derive(Deserialize, Debug)]
pub struct TrackItem {
    pub name: String,
    pub duration_ms: i32,
    pub id: String,
    pub album: AlbumItem,
}
#[derive(Deserialize, Debug)]
pub struct AlbumItem {
    pub images: Vec<Image>,
}
