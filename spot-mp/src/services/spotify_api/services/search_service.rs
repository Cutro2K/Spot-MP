use futures::future::{try_join, try_join_all};

use crate::{
    domain::{
        album::Album,
        playlist::{Playlist, PlaylistOrigin, SpotifyPlaylistData, SpotifySearchResponse},
        song::{Song, SongOrigin, SpotifySongData},
        spot_mp_errors::SpotMpError,
    },
    services::spotify_api::{
        client::http_client::SpotifyHttpClient,
        models::{AlbumItem, PlaylistSearchItem, SearchQueryResponse, TrackItem},
        services::{
            album_service::AlbumService, artist_service::ArtistService,
            playlist_service::PlaylistService, utils::parse_release_year,
        },
        traits::{AlbumManager, PlaylistManager, SearchManager},
    },
};

pub struct SearchService {
    http: SpotifyHttpClient,
}

impl SearchService {
    pub fn new(http: SpotifyHttpClient) -> Self {
        Self { http }
    }

    fn format_tracks(items: Vec<TrackItem>) -> Vec<SpotifySearchResponse> {
        items
            .into_iter()
            .map(|item| {
                let artist = ArtistService::format_artists(&item.artists);
                let album_name = item.album.name;
                let release_year = parse_release_year(item.album.release_date.as_deref());
                let img_url = item
                    .album
                    .images
                    .first()
                    .map(|img| img.url.clone())
                    .unwrap_or_default();

                SpotifySearchResponse::Song(Song::with_metadata(
                    None,
                    item.name,
                    artist,
                    album_name,
                    release_year,
                    (item.duration_ms / 1000).into(),
                    img_url,
                    SongOrigin::Spotify(SpotifySongData {
                        spotify_id: item.id,
                    }),
                ))
            })
            .collect()
    }

    async fn fetch_albums(&self, items: Vec<AlbumItem>) -> Result<Vec<Album>, SpotMpError> {
        let album_service = AlbumService::new(self.http.clone());
        let mut future_petitions = Vec::new();

        for item in items {
            let name = item.name;
            let artist = ArtistService::format_artists(&item.artists);
            let album_id = item.id;
            let release_year = parse_release_year(item.release_date.as_deref());
            let img_path = item
                .images
                .first()
                .map(|img| img.url.clone())
                .unwrap_or_default();

            let album_svc = &album_service;
            future_petitions.push(async move {
                let songs = album_svc
                    .get_album_tracks(&album_id, &name, release_year, &img_path)
                    .await?;

                Ok::<Album, SpotMpError>(Album::new(
                    album_id,
                    name,
                    artist,
                    release_year,
                    img_path,
                    songs,
                ))
            });
        }

        try_join_all(future_petitions).await
    }

    async fn fetch_playlists(
        &self,
        items: Vec<Option<PlaylistSearchItem>>,
    ) -> Result<Vec<Playlist>, SpotMpError> {
        let playlist_service = PlaylistService::new(self.http.clone());
        let mut future_petitions = Vec::new();

        for item in items.into_iter().flatten() {
            let name = item.name;
            let playlist_id = item.id;
            let img_url = item.images.first().map(|img| img.url.clone());

            let playlist_svc = &playlist_service;
            future_petitions.push(async move {
                let songs = playlist_svc.get_playlist_tracks(&playlist_id).await?;

                Ok::<Playlist, SpotMpError>(Playlist::new(
                    songs,
                    false,
                    name,
                    img_url,
                    PlaylistOrigin::Spotify(SpotifyPlaylistData {
                        id_playlist: playlist_id,
                    }),
                ))
            });
        }

        try_join_all(future_petitions).await
    }
}

impl SearchManager for SearchService {
    async fn search_query(
        &self,
        query: &str,
    ) -> Result<Vec<SpotifySearchResponse>, SpotMpError> {
        let response = self
            .http
            .get("https://api.spotify.com/v1/search")
            .query(&[
                ("q", query),
                ("type", "track,album,playlist"),
                ("limit", "3"),
                ("offset", "0"),
            ])
            .send_json::<SearchQueryResponse>()
            .await?;

        let mut result = Self::format_tracks(response.tracks.items);

        let (albums, playlists) = try_join(
            self.fetch_albums(response.albums.items),
            self.fetch_playlists(response.playlists.items),
        )
        .await?;

        result.extend(albums.into_iter().map(SpotifySearchResponse::Album));
        result.extend(playlists.into_iter().map(SpotifySearchResponse::Playlist));

        Ok(result)
    }
}