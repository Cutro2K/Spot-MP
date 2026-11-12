use serde::Deserialize;

// --- Structs for responses using serde to deserialize json files ---

// --- Login & User Profile ---
#[derive(Deserialize, Debug)]
pub struct SpotifyTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i32,
    pub refresh_token: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct UserProfileResponse {
    pub id: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ArtistItem {
    pub name: String,
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
    #[serde(default)]
    pub collaborative: bool,
    pub id: String,
    #[serde(default)]
    pub images: Vec<Image>,
    pub owner: Owner,
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct Owner {
    pub id: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Image {
    pub url: String,
    pub height: Option<i32>,
    pub width: Option<i32>,
}

// -- Get Playlist Songs --
#[derive(Deserialize, Debug)]
pub struct GetPlaylistSongsResponse {
    pub next: Option<String>,
    pub items: Vec<PlaylistSongItem>,
}

#[derive(Deserialize, Debug)]
pub struct PlaylistSongItem {
    #[serde(alias = "item")]
    pub track: Option<PlaylistTrackItem>,
}

#[derive(Deserialize, Debug)]
pub struct PlaylistTrackItem {
    pub name: String,
    pub duration_ms: i32,
    pub id: Option<String>,
    #[serde(default)]
    pub artists: Vec<ArtistItem>,
    pub album: AlbumItem,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AlbumItem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub images: Vec<Image>,
    #[serde(default)]
    pub artists: Vec<ArtistItem>,
    pub release_date: Option<String>,
}

// --- Import Album Function Response ---
#[derive(Deserialize, Debug)]
pub struct GetUserAlbumsRespose {
    pub next: Option<String>,
    pub items: Vec<SavedAlbumObject>,
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
    #[serde(default)]
    pub artists: Vec<ArtistItem>,
}

// --- Search Song Response ---
#[derive(Deserialize, Debug)]
pub struct SearchQueryResponse {
    pub tracks: SearchTracks,
    pub albums: SearchAlbums,
    pub playlists: SearchPlaylists,
}

#[derive(Deserialize, Debug)]
pub struct SearchTracks {
    pub next: Option<String>,
    pub items: Vec<TrackItem>,
}

#[derive(Deserialize, Debug)]
pub struct SearchAlbums {
    pub next: Option<String>,
    pub items: Vec<AlbumItem>,
}

#[derive(Deserialize, Debug)]
pub struct SearchPlaylists {
    pub next: Option<String>,
    pub items: Vec<Option<PlaylistSearchItem>>,
}

#[derive(Deserialize, Debug)]
pub struct PlaylistSearchItem {
    pub name: String,
    pub id: String,
    #[serde(default)]
    pub images: Vec<Image>,
}

#[derive(Deserialize, Debug)]
pub struct TrackItem {
    pub name: String,
    pub duration_ms: i32,
    pub id: String,
    #[serde(default)]
    pub artists: Vec<ArtistItem>,
    pub album: AlbumItem,
}
