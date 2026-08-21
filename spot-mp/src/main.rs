use log::info;
use crate::data_storage::file_manager::FileManager;
pub mod data_storage;
pub mod spotify_api;
pub mod domain;
pub mod audio_engine;
pub mod dispatcher;
pub mod app;
pub mod controllers;

fn main() {
    FileManager::init_logger();
    info!("Spot-MP is starting...");
    info!("Spot-MP finished correctly! :)");
}
