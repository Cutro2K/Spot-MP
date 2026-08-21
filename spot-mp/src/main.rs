use log::info;
pub mod data_storage;
pub mod spotify_api;
pub mod domain;
pub mod audio_engine;
pub mod dispatcher;
pub mod app;
pub mod controllers;

fn main() {
    info!("Spot-MP is starting...");
}
