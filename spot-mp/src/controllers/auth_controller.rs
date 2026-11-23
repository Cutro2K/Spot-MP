use log::{error, info};
use std::path::PathBuf;

use crate::{
    domain::auth_models::{SessionData, WebApiData},
    services::data_storage::auth_storage::AuthStorage,
};

#[allow(dead_code)]
pub struct AuthController {
    session_data: SessionData,
    auth_storage: AuthStorage,
}

impl AuthController {
    pub fn new(spot_mp_folder_path: PathBuf) -> Self {
        let auth_storage = AuthStorage::new(spot_mp_folder_path);
        let session_data = auth_storage.load_session().unwrap_or_default();
        Self {
            session_data,
            auth_storage,
        }
    }

    pub fn load_session(&mut self) -> SessionData {
        match self.auth_storage.load_session() {
            Ok(session_data) => {
                info!("The saved data loaded successfully!");
                self.session_data = session_data.clone();
                session_data
            }
            Err(e) => {
                error!("Failed retrieving saved credentials: {}", e);
                info!("Loading default values...");
                SessionData::default()
            }
        }
    }

    pub fn save_session(&mut self, new_data: SessionData) {
        if let Err(e) = self.auth_storage.save_session(new_data.clone()) {
            error!("Failed saving the app credentials: {}", e);
            return;
        }
        self.session_data = new_data;
        info!("The session data was saved successfully!");
    }

    pub fn erase_session(&mut self) {
        let default_session = SessionData::default();
        if let Err(e) = self.auth_storage.save_session(default_session.clone()) {
            error!("Failed to erase session data saving default values: {}", e);
            return;
        }
        self.session_data = default_session;
        info!("The session was deleted successfully!");
    }

    pub fn get_session(&self) -> SessionData {
        self.session_data.clone()
    }

    pub fn sync_web_api(&mut self, web_api: Option<WebApiData>) {
        if self.session_data.web_api != web_api {
            let mut updated = self.session_data.clone();
            updated.web_api = web_api;
            self.save_session(updated);
        }
    }
}
