use crate::services::spotify_api::models::ArtistItem;

pub struct ArtistService {}

impl ArtistService {
    pub fn format_artists(artists: &[ArtistItem]) -> String {
        if artists.is_empty() {
            String::from("Unknown Artist")
        } else {
            artists
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        }
    }
}