use std::sync::atomic::{AtomicBool, Ordering};

use log::{error, info};

use crate::{
    domain::{auth_models::WebApiData, song::Song, spot_mp_errors::SpotMpError},
    services::spotify_api::{
        auth::auth_service::AuthService,
        services::{
            album_service::AlbumService, playlist_service::PlaylistService,
            search_service::SearchService, user_service::UserService,
        },
        spotify_api_client::SpotifyApiClient,
        traits::{AlbumManager, AuthManager, PlaylistManager, SearchManager, UserManager},
    },
    state_store::app::action::{Action, SpotifyAction},
};

pub struct SpotifyController<
    A: AuthManager = AuthService,
    U: UserManager = UserService,
    P: PlaylistManager = PlaylistService,
    Al: AlbumManager = AlbumService,
    S: SearchManager = SearchService,
> {
    client_id: Option<String>,
    logged_in: AtomicBool,
    client: SpotifyApiClient<A, U, P, Al, S>,
}

impl Default for SpotifyController {
    fn default() -> Self {
        Self::new()
    }
}

impl SpotifyController {
    pub fn new() -> Self {
        Self::with_client(SpotifyApiClient::new())
    }
}

impl<A, U, P, Al, S> SpotifyController<A, U, P, Al, S>
where
    A: AuthManager,
    U: UserManager,
    P: PlaylistManager,
    Al: AlbumManager,
    S: SearchManager,
{
    pub fn with_client(client: SpotifyApiClient<A, U, P, Al, S>) -> Self {
        Self {
            client_id: None,
            logged_in: AtomicBool::new(false),
            client,
        }
    }

    pub fn restore_session(&mut self, client_id: String, web_api: WebApiData) {
        self.client_id = Some(client_id.clone());
        self.client
            .restore_session(client_id, web_api.access_token, web_api.refresh_token);
        self.logged_in.store(true, Ordering::SeqCst);
    }

    pub fn client_id(&self) -> Option<String> {
        self.client_id.clone()
    }

    pub fn is_logged_in(&self) -> bool {
        self.logged_in.load(Ordering::SeqCst)
    }

    pub fn tokens(&self) -> Option<WebApiData> {
        let access_token = self.client.access_token()?;
        let refresh_token = self.client.refresh_token()?;
        Some(WebApiData {
            access_token,
            refresh_token,
        })
    }

    pub async fn login(&mut self, client_id: String) -> Result<(), SpotMpError> {
        info!("Logging in to Spotify with client id: {}...", client_id);
        if let Err(e) = self.client.login(&client_id).await {
            error!("Spotify Error (Login): {}", e);
            return Err(e);
        }
        self.client_id = Some(client_id);
        self.logged_in.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn logout(&mut self) -> Action {
        self.client.logout();
        self.logged_in.store(false, Ordering::SeqCst);
        self.client_id = None;
        Action::Spotify(SpotifyAction::LoggedOut)
    }

    fn handle_spotify_error(&self, err: SpotMpError, context: &str, user_msg: &str) -> Action {
        match err {
            SpotMpError::SessionExpired(reason) => {
                error!("Spotify Session Expired ({}): {}", context, reason);
                self.logged_in.store(false, Ordering::SeqCst);
                Action::Spotify(SpotifyAction::SessionExpired)
            }
            other => {
                error!("Spotify Error ({}): {}", context, other);
                Action::UnexpectedErrorHappened(user_msg.to_string())
            }
        }
    }

    pub async fn fetch_playlists(&self) -> Action {
        if !self.is_logged_in() {
            return Action::NoOp;
        }
        if self.client_id.is_none() {
            error!("Tried to fetch playlists when the user is logged out!");
            return Action::Spotify(SpotifyAction::UserIsLoggedOut);
        }

        match self.client.import_liked_playlist().await {
            Ok(playlists) => Action::Spotify(SpotifyAction::FetchedPlaylists(playlists)),
            Err(e) => self.handle_spotify_error(
                e,
                "fetch_playlists",
                "Failed to import liked playlists!",
            ),
        }
    }

    pub async fn fetch_albums(&self) -> Action {
        if !self.is_logged_in() {
            return Action::NoOp;
        }
        if self.client_id.is_none() {
            error!("Tried to fetch albums when the user is logged out!");
            return Action::Spotify(SpotifyAction::UserIsLoggedOut);
        }

        match self.client.import_liked_albums().await {
            Ok(albums) => Action::Spotify(SpotifyAction::FetchedAlbums(albums)),
            Err(e) => {
                self.handle_spotify_error(e, "fetch_albums", "Failed to import liked albums!")
            }
        }
    }

    pub async fn search_query(&self, query: &str) -> Action {
        if !self.is_logged_in() {
            return Action::NoOp;
        }
        if self.client_id.is_none() {
            error!("Tried to process a query when the user is logged out!");
            return Action::Spotify(SpotifyAction::UserIsLoggedOut);
        }

        match self.client.search_query(query).await {
            Ok(result) => Action::Spotify(SpotifyAction::QueryResult(result)),
            Err(e) => {
                self.handle_spotify_error(e, "search a query", "Failed to process the query!")
            }
        }
    }

    pub async fn add_song_to_playlist(&self, playlist_id: &str, song: Song) -> Action {
        if !self.is_logged_in() {
            return Action::NoOp;
        }
        if self.client_id.is_none() {
            error!("Tried to add a song to a playlist when the user is logged out!");
            return Action::Spotify(SpotifyAction::UserIsLoggedOut);
        }

        let song_id = song.song_data();
        match self.client.add_song_to_playlist(playlist_id, song_id).await {
            Ok(_) => Action::Spotify(SpotifyAction::AddedSongTo(song, playlist_id.to_string())),
            Err(e) => self.handle_spotify_error(
                e,
                "add song to playlist",
                "Failed to add song to Spotify playlist!",
            ),
        }
    }

    pub async fn remove_song_from_playlist(&self, playlist_id: &str, song: Song) -> Action {
        if !self.is_logged_in() {
            return Action::NoOp;
        }
        if self.client_id.is_none() {
            error!("Tried to remove a song from a playlist when the user is logged out!");
            return Action::Spotify(SpotifyAction::UserIsLoggedOut);
        }

        let song_id = song.song_data();

        match self
            .client
            .remove_song_from_playlist(playlist_id, song_id)
            .await
        {
            Ok(_) => Action::Spotify(SpotifyAction::RemovedSongFrom(
                song,
                playlist_id.to_string(),
            )),
            Err(e) => self.handle_spotify_error(
                e,
                "remove song from playlist",
                "Failed to remove song from Spotify playlist!",
            ),
        }
    }

    pub async fn create_playlist(
        &self,
        playlist_name: &str,
        public: bool,
        opt_song: Option<Song>,
    ) -> Action {
        if !self.is_logged_in() {
            return Action::NoOp;
        }
        if self.client_id.is_none() {
            error!("Tried to create a playlist when the user is logged out!");
            return Action::Spotify(SpotifyAction::UserIsLoggedOut);
        }

        match self
            .client
            .create_playlist(playlist_name, public, opt_song.as_ref())
            .await
        {
            Ok(playlist) => {
                Action::Spotify(SpotifyAction::PlaylistCreatedWith(playlist, opt_song))
            }
            Err(e) => self.handle_spotify_error(
                e,
                "create playlist",
                "Failed to create the playlist!",
            ),
        }
    }

    pub async fn delete_playlist(&self, playlist_id: &str) -> Action {
        if !self.is_logged_in() {
            return Action::NoOp;
        }
        if self.client_id.is_none() {
            error!("Tried to delete a playlist when the user is logged out!");
            return Action::Spotify(SpotifyAction::UserIsLoggedOut);
        }

        match self.client.unfollow_playlist(playlist_id).await {
            Ok(_) => Action::Spotify(SpotifyAction::DeletedPlaylist(playlist_id.to_string())),
            Err(e) => self.handle_spotify_error(
                e,
                "delete playlist",
                "Failed to delete the playlist!",
            ),
        }
    }

    pub async fn delete_album(&self, album_id: &str) -> Action {
        if !self.is_logged_in() {
            return Action::NoOp;
        }
        if self.client_id.is_none() {
            error!("Tried to delete an album when the user is logged out!");
            return Action::Spotify(SpotifyAction::UserIsLoggedOut);
        }

        match self.client.unfollow_album(album_id).await {
            Ok(_) => Action::Spotify(SpotifyAction::DeletedAlbum(album_id.to_string())),
            Err(e) => {
                self.handle_spotify_error(e, "delete album", "Failed to delete the album!")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controllers::test_doubles::{
        StubAlbum, StubAuth, StubPlaylist, StubSearch, StubUser,
    };
    use crate::domain::spot_mp_errors::Status;

    #[tokio::test]
    async fn session_expired_maps_to_action_marks_logged_out_and_preserves_client_id() {
        let auth = StubAuth::default();
        auth.push_renew_outcome(Err(SpotMpError::SpotifyErrorStatus(Status::BadRequest(
            "invalid_grant".to_string(),
        ))));
        let playlist = StubPlaylist::with_responses(vec![Err(SpotMpError::SpotifyErrorStatus(
            Status::Unauthorized("expired".to_string()),
        ))]);
        let client = SpotifyApiClient::with_services(
            auth,
            StubUser,
            playlist,
            StubAlbum,
            StubSearch::with_responses(vec![]),
        );

        let mut controller = SpotifyController::with_client(client);
        controller.restore_session(
            "client-123".to_string(),
            WebApiData {
                access_token: "access-v1".to_string(),
                refresh_token: "refresh-v1".to_string(),
            },
        );

        let shared: &SpotifyController<_, _, _, _, _> = &controller;
        let action = shared.fetch_playlists().await;

        assert!(
            matches!(action, Action::Spotify(SpotifyAction::SessionExpired)),
            "expected SessionExpired action, got: {action:?}"
        );
        assert!(!controller.is_logged_in());
        assert_eq!(controller.client_id().as_deref(), Some("client-123"));
        assert!(controller.tokens().is_none());

        // Subsequent calls while logged out return NoOp instead of hitting the API.
        let follow_up = shared.fetch_playlists().await;
        assert!(matches!(follow_up, Action::NoOp));
    }

    #[tokio::test]
    async fn non_expiry_errors_keep_session_intact_and_refresh_succeeds_via_shared_self() {
        let auth = StubAuth::default();
        // 1st call: 401 -> refreshes to v2 -> retry succeeds.
        // 2nd call: 401 -> refreshes to v3 -> retry still returns 401 (scope/permission error).
        // 3rd call: 500 server error (no refresh).
        auth.push_renew_outcome(Ok(("access-v2".to_string(), "refresh-v2".to_string())));
        auth.push_renew_outcome(Ok(("access-v3".to_string(), "refresh-v3".to_string())));

        let playlist = StubPlaylist::with_responses(vec![
            Err(SpotMpError::SpotifyErrorStatus(Status::Unauthorized(
                "expired".to_string(),
            ))),
            Ok(vec![]),
        ]);
        let search = StubSearch::with_responses(vec![
            Err(SpotMpError::SpotifyErrorStatus(Status::Unauthorized(
                "expired".to_string(),
            ))),
            Err(SpotMpError::SpotifyErrorStatus(Status::Unauthorized(
                "insufficient scope".to_string(),
            ))),
            Err(SpotMpError::SpotifyErrorStatus(Status::Unknown(
                500,
                "internal error".to_string(),
            ))),
        ]);

        let client = SpotifyApiClient::with_services(auth, StubUser, playlist, StubAlbum, search);
        let mut controller = SpotifyController::with_client(client);
        controller.restore_session(
            "client-123".to_string(),
            WebApiData {
                access_token: "access-v1".to_string(),
                refresh_token: "refresh-v1".to_string(),
            },
        );

        let shared: &SpotifyController<_, _, _, _, _> = &controller;

        // &self fetch_playlists transparently refreshes and succeeds
        let fetched = shared.fetch_playlists().await;
        assert!(matches!(
            fetched,
            Action::Spotify(SpotifyAction::FetchedPlaylists(_))
        ));
        assert!(controller.is_logged_in());
        assert_eq!(
            controller.tokens(),
            Some(WebApiData {
                access_token: "access-v2".to_string(),
                refresh_token: "refresh-v2".to_string(),
            })
        );

        // Double-401 maps to UnexpectedErrorHappened without logging out or clearing tokens
        let double_401 = shared.search_query("daft punk").await;
        assert!(matches!(double_401, Action::UnexpectedErrorHappened(_)));
        assert!(controller.is_logged_in());
        assert_eq!(controller.client_id().as_deref(), Some("client-123"));
        assert_eq!(
            controller.tokens(),
            Some(WebApiData {
                access_token: "access-v3".to_string(),
                refresh_token: "refresh-v3".to_string(),
            })
        );

        // 5xx maps to UnexpectedErrorHappened without logging out or clearing tokens
        let server_err = shared.search_query("daft punk").await;
        assert!(matches!(server_err, Action::UnexpectedErrorHappened(_)));
        assert!(controller.is_logged_in());
        assert_eq!(controller.client_id().as_deref(), Some("client-123"));
        assert_eq!(
            controller.tokens(),
            Some(WebApiData {
                access_token: "access-v3".to_string(),
                refresh_token: "refresh-v3".to_string(),
            })
        );
    }
}
