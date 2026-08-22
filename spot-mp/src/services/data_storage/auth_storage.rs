use std::{fs::{File, OpenOptions}, io::{Read, Write}, path::PathBuf};

use crate::domain::{auth_models::SessionData, spot_mp_errors::SpotMpError};

pub struct AuthStorage {
    file_path: PathBuf,
}

impl AuthStorage {
    pub fn new(spot_mp_folder_path: PathBuf) -> Self {
        let file_path = spot_mp_folder_path.join("session.json");
        Self { file_path }
    }

    pub fn load_session(&self) -> Result<SessionData, SpotMpError> {
        if !self.file_path.exists() {
            return Ok(SessionData::default());
        }

        let mut file = File::open(&self.file_path)?;
        let mut content = String::new();

        file.read_to_string(&mut content)?;
        
        Ok(serde_json::from_str(&content)?)
    }

    pub fn save_session(&self, data: SessionData) -> Result<(), SpotMpError> {
        let json_data = serde_json::to_string(&data)?;
        
        let mut file = OpenOptions::new().write(true).read(true).truncate(true).create(true).open(&self.file_path)?;

        file.write_all(json_data.as_bytes())?;
        Ok(())
    }
}
