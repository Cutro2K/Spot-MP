// This class provides a available port for callbacks binding an address
pub struct OAuthConfig {
    pub host: String,
    pub port: u16,
    pub callback_path: String,
}

impl Default for OAuthConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8888,
            callback_path: "/callback".to_string(),
        }
    }
}

impl OAuthConfig {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            ..Default::default()
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port) 
    }

    pub fn redirect_uri(&self) -> String {
        format!("http://{}:{}{}", self.host, self.port, self.callback_path) 
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}