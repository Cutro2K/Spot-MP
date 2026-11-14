use crate::{
    domain::spot_mp_errors::SpotMpError,
    services::spotify_api::traits::BrowserOpener,
};

pub struct SystemBrowserOpener;

impl BrowserOpener for SystemBrowserOpener {
    fn open(&self, url: &str) -> Result<(), SpotMpError> {
        webbrowser::open(url)
            .map_err(|e| SpotMpError::Error(format!("Failed to open browser: {}", e)))
    }
}