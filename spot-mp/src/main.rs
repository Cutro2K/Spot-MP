use log::info;
use crate::services::data_storage::log_service::LogService;
pub mod domain;
pub mod controllers;
pub mod services;
pub mod ui_manager;
pub mod state_store;

fn main() {
    LogService::init();
    info!("Spot-MP is starting...");
    info!("Spot-MP finished correctly! :)");
}
