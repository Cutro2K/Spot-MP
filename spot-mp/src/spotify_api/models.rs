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

// -- Import Playlist Function Response ---
#[derive(Deserialize, Debug)]
pub struct GetUserPlaylistsResponse {
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

// --  Get Playlist Songs --

#[derive(Deserialize, Debug)]
pub struct GetPlaylistSongsResponse { 
    pub next: Option<String>,
    pub items: Vec<PlaylistSongItem>,
}
#[derive(Deserialize, Debug)]
pub struct PlaylistSongItem { // Each one of this it's a track added to a playlist
    pub item: PlaylistTrackItem,
}
#[derive(Deserialize, Debug)]
pub struct PlaylistTrackItem { // Each one of this is a single track from a  playlist
    pub name: String,
    pub duration_ms: i32,
    pub id: String,
    pub album: AlbumItem,
}
#[derive(Deserialize, Debug)]
pub struct AlbumItem {
    pub id: String,
    pub name: String, 
    pub images: Vec<Image>,
}

// --- Import Album Function Response ---
#[derive(Deserialize, Debug)]
pub struct GetUserAlbumsRespose {
    pub next: Option<String>,
    pub items: Vec<SavedAlbumObject>
}
#[derive(Deserialize, Debug)]
pub struct SavedAlbumObject {
    pub album: AlbumItem,
}
// -- Get Album Songs --
#[derive(Deserialize, Debug)]
pub struct GetAlbumSongsResponse {
    pub next: Option<String>,
    pub items: Vec<AlbumTrackItem>,
}
#[derive(Deserialize, Debug)]
pub struct AlbumTrackItem {
    pub name: String,
    pub duration_ms: i32,
    pub id: String,
}
