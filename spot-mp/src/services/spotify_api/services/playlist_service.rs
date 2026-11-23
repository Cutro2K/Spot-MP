use futures::future::try_join_all;
use serde_json::json;

use crate::{domain::{playlist::{Playlist, PlaylistOrigin, SpotifyPlaylistData}, song::{Song, SongOrigin, SpotifySongData}, spot_mp_errors::SpotMpError}, services::spotify_api::{client::http_client::SpotifyHttpClient, models::{GetPlaylistSongsResponse, GetUserPlaylistsResponse, PlaylistItem}, services::{artist_service::ArtistService, utils::parse_release_year}, traits::PlaylistManager}};


pub struct PlaylistService {
    http: SpotifyHttpClient,
}

impl PlaylistService {
    pub fn new(http: SpotifyHttpClient) -> Self {
        Self {
            http,
        }
    }
}

impl PlaylistManager for PlaylistService {
    async fn create_playlist(
        &self,
        playlist_name: &str,
        public: bool,
    ) -> Result<Playlist, SpotMpError> {
        let url = "https://api.spotify.com/v1/me/playlists";

        let body = json!({
            "name": playlist_name,
            "public": public,
        });

        let playlist_data = self
            .http
            .post(url)
            .json(&body)
            .send_json::<PlaylistItem>()
            .await?;

        Ok(Playlist::new(
            Vec::new(),
            true,
            playlist_data.name,
            None,
            PlaylistOrigin::Spotify(SpotifyPlaylistData {
                id_playlist: playlist_data.id,
            }),
        ))
    }

    async fn add_song_to_playlist(
        &self, 
        playlist_id: &str, 
        song_id: &str
    ) -> Result<(), SpotMpError>
    {
        let url = format!(
            "https://api.spotify.com/v1/playlists/{}/items", 
            playlist_id
        );

        let track_uri = if song_id.starts_with("spotify:track:") {
            song_id.to_string()
        } else {
            format!("spotify:track:{}", song_id)
        };

        let body = json!({
            "uris": [track_uri],
        });

        self.http.post(&url).json(&body).send_empty().await
    }

    async fn remove_song_from_playlist(
        &self,
        playlist_id: &str,
        song_id: &str,
    ) -> Result<(), SpotMpError>
    {
        let url = format!(
            "https://api.spotify.com/v1/playlists/{}/items", 
            playlist_id
        );

        let track_uri = if song_id.starts_with("spotify:track:") {
            song_id.to_string()
        } else {
            format!("spotify:track:{}", song_id)
        };

        let uris = [track_uri];
        let items: Vec<_> = uris.iter().map(|u| json!({"uri": u})).collect(); 

        let body = json!({
            "items": items,
        });

        self.http.delete(&url).json(&body).send_empty().await
    }

    async fn unfollow_playlist(
        &self, 
        playlist_id: &str
    ) -> Result<(), SpotMpError>
    {
        let url = "https://api.spotify.com/v1/me/library";

        let uri = if playlist_id.starts_with("spotify:playlist:") {
            playlist_id.to_string()
        } else {
            format!("spotify:playlist:{}", playlist_id)
        };

        self.http
            .delete(url)
            .query(&[("uris", &uri)])
            .send_empty()
            .await
    }

    async fn get_playlist_tracks(
        &self, 
        playlist_id: &str
    ) -> Result<Vec<Song>, SpotMpError>
    {
        let mut result: Vec<Song> = Vec::new();
        let mut next_url = Some(format!(
            "https://api.spotify.com/v1/playlists/{}/items?limit=50",
            playlist_id
        ));

        while let Some(url) = next_url {
            let response = self.http.get(&url).send_json::<GetPlaylistSongsResponse>().await?;
            
            for item in response.items {
                if let Some(track) = item.track 
                && let Some(track_id) = track.id {
                    let artist = ArtistService::format_artists(&track.artists);
                    let album_name = track.album.name.clone();
                    let release_year = parse_release_year(
                        track.album.release_date.as_deref()
                    );
                    let img_url = track
                        .album
                        .images
                        .first()
                        .map(|img| img.url.clone())
                        .unwrap_or_default();
                 
                    result.push(
                        Song::with_metadata(
                            None, 
                            track.name, 
                            artist, 
                            album_name, 
                            release_year, 
                            (track.duration_ms/1000).into(), 
                            img_url,
                            SongOrigin::Spotify(SpotifySongData {
                                spotify_id: track_id,
                            }),
                        )
                    );            
                 }
            }
            next_url = response.next;
        }
        Ok(result)
    }

    async fn import_liked_playlist(
        &self,
        user_id: &str
    ) -> Result<Vec<Playlist>, SpotMpError>
    {
        let mut result: Vec<Playlist> = Vec::new();
        let mut next_url = Some("https://api.spotify.com/v1/me/playlists?limit=10".to_string());

        while let Some(url) = next_url {
            let playlist_response = self.http.get(&url).send_json::<GetUserPlaylistsResponse>().await?;
            let mut future_petitions = Vec::new();
            
            for item in playlist_response.items {
                let name = item.name.clone();
                let is_modifiable = item.collaborative || item.owner.id == user_id;
                let playlist_id = item.id;
                let img_url = item.images.first().map(|img| img.url.clone());

                future_petitions.push(async move {
                    let songs = self.get_playlist_tracks(&playlist_id).await?;

                    Ok::<Playlist, SpotMpError>(Playlist::new(
                        songs,
                        is_modifiable,
                        name,
                        img_url,
                        PlaylistOrigin::Spotify(SpotifyPlaylistData {
                            id_playlist: playlist_id,
                        }),
                    ))
                });
            }

            let processed_playlist = try_join_all(future_petitions).await?;
            result.extend(processed_playlist);

            next_url = playlist_response.next;
        }
        Ok(result)   
    }
}