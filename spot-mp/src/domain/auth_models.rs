use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub struct SessionData {
    pub web_api: Option<WebApiData>,
    pub librespot: Option<LibrespotData>,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub struct WebApiData {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub struct LibrespotData {
    pub device_id: String,
    pub credentials: String,
}
