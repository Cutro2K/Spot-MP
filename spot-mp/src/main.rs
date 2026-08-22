use log::info;
use crate::services::data_storage::file_manager::FileManager;
pub mod domain;
pub mod controllers;
pub mod services;
pub mod ui_manager;
pub mod state_store;

fn main() {
    FileManager::init_logger();
    info!("Spot-MP is starting...");
    info!("Spot-MP finished correctly! :)");
}
