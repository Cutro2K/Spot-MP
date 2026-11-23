pub mod auth_controller;
pub mod controller_hub;
pub mod local_controller;
pub mod music_player_controller;
pub mod spotify_controller;
#[cfg(test)]
pub(crate) mod test_doubles;
#[cfg(test)]
mod spotify_http_integration_tests;
