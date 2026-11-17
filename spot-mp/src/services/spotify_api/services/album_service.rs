use futures::future::try_join_all;

use crate::{domain::{album::Album, song::{Song, SongOrigin, SpotifySongData}, spot_mp_errors::SpotMpError}, services::spotify_api::{client::http_client::SpotifyHttpClient, models::{GetAlbumSongsResponse, GetUserAlbumsRespose}, services::{artist_service::ArtistService, utils::parse_release_year}, traits::AlbumManager}};

pub struct AlbumService {
    http: SpotifyHttpClient,
}

impl AlbumService {
    pub fn new(http: SpotifyHttpClient) -> Self {
        Self {
            http: http
        }
    }
}

impl AlbumManager for AlbumService {
    async fn get_album_tracks(
        &self,
        album_id: &str,
        album_name: &str,
        release_year: Option<i32>,
        img_path: &str,
    ) -> Result<Vec<Song>, SpotMpError>
    {
        let mut result: Vec<Song> = Vec::new();
        let mut next_url = Some(format!(
            "https://api.spotify.com/v1/albums/{}/tracks?limit=50",
            album_id
        ));

        while let Some(url) = next_url {
            let response = self.http.get(&url).send_json::<GetAlbumSongsResponse>().await?;
            
            for item in response.items {
                let artist = ArtistService::format_artists(&item.artists);
                result.push(Song::with_metadata(
                    None,
                    item.name,
                    artist,
                    album_name.to_owned(),
                    release_year,
                    (item.duration_ms / 1000).into(),
                    img_path.to_owned(),
                    SongOrigin::Spotify(SpotifySongData {
                        spotify_id: item.id,
                    }),
                ));
            }
            next_url = response.next;
        }
        Ok(result)
    }

    async fn import_liked_albums(
        &self
    ) -> Result<Vec<Album>, SpotMpError>
    {
        let mut result: Vec<Album> = Vec::new();
        let mut next_url = Some("https://api.spotify.com/v1/me/albums?limit=10".to_string());

        while let Some(url) = next_url {
            let album_response = self.http.get(&url).send_json::<GetUserAlbumsRespose>().await?;

            let mut future_petitions = Vec::new();

            for item in album_response.items {
                let name = item.album.name.clone();
                
                let album_id = item.album.id;

                let artists = ArtistService::format_artists(&item.album.artists);

                let release_year =
                    parse_release_year(item.album.release_date.as_deref());
                
                let img_path = item
                    .album
                    .images
                    .first()
                    .map(|img| img.url.clone())
                    .unwrap_or_default();

                future_petitions.push(async move {
                    let songs = self
                        .get_album_tracks(&album_id, &name, release_year, &img_path)
                        .await?;

                    Ok::<Album, SpotMpError>(Album::new(
                        album_id,
                        name,
                        artists,
                        release_year,
                        img_path,
                        songs
                    ))
                });
            }

            let processed_playlist = try_join_all(future_petitions).await?;
            result.extend(processed_playlist);
            
            next_url = album_response.next;
        }

        Ok(result)
    }

    async fn unfollow_album(
        &self, 
        album_id: &str
    ) -> Result<(), SpotMpError>
    {
        let url = "https://api.spotify.com/v1/me/library";

        let uri = if album_id.starts_with("spotify:album:") {
            album_id.to_string()
        } else {
            format!("spotify:album:{}", album_id)
        };
        
        self.http
            .delete(url)
            .query(&[("uris", &uri)])
            .send_empty()
            .await
    }
}