use pkce;

pub struct SpotifyAuth {
    code_verifier: String, 
    code_challenge: String,
}

impl Default for SpotifyAuth {
    fn default() -> Self {
        Self::new()
    }
}
impl SpotifyAuth {
    pub fn new() -> Self {
        let code_verifier = pkce::code_verifier(128);
        let code_challenge = pkce::code_challenge(&code_verifier);
        let code_verifier = String::from_utf8_lossy(&code_verifier).to_string();
        Self {
            code_verifier,
            code_challenge,
        }
    }
    pub fn code_verifier(&self) -> &str {
        self.code_verifier.as_ref()
    }
    pub fn code_challenge(&self) -> &str {
        self.code_challenge.as_ref()
    }
}
