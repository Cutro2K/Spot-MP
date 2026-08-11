use keyring::{Entry};
use crate::domain::spot_mp_errors::SpotMpError;

pub struct AuthStorage {
    entry: Entry,
}

impl AuthStorage {
    pub fn new(service_name: &str, user_name: &str) -> Result<Self, SpotMpError> {
        let entry = Entry::new(service_name, user_name)?; 
        Ok(AuthStorage {entry})
    }

    pub fn get_refresh_token(&self) -> Result<String, SpotMpError>{
        Ok(self.entry.get_password()?)
    }
    pub fn save_refresh_token(&self, token: String) -> Result<(), SpotMpError>{
        Ok(self.entry.set_password(&token)?)
    }
    pub fn delete_refresh_token(&self) -> Result<(), SpotMpError>{
        Ok(self.entry.delete_credential()?)
    }
    
}


#[cfg(test)]
mod tests {
    use super::*;
    use keyring_core::mock; 

    fn setup_mock_keyring() {
        let mock_store = mock::Store::new().expect("Failed to create the mock store for the keyring");
        keyring_core::set_default_store(mock_store);
    }

    #[test]
    #[ignore]
    fn test_auth_storage_lifecycle() {
        setup_mock_keyring();

        let _ = AuthStorage::new("spot_mp_test_service", "test_user")
            .expect("Failed to initialice the AuthStorage");

        keyring_core::unset_default_store();
    }
}
