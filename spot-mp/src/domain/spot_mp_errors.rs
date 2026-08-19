use std::{io};
use sqlite;
use audiotags;
use reqwest;    
use url::ParseError;
use std::fmt::{self};
use rodio::decoder::DecoderError;
use rodio::source::SeekError;
use librespot::core;
use serde_json;

#[derive(Debug)]
pub enum SpotMpError {
    /// --- General ---
    Error(String),
    
    /// --- Data & Storage ---
    // Data base errors used in data_storage/db_controller.rs
    SQLite(sqlite::Error),

    // Audiotags error used in data_storage/file_manager.rs
    AudioTag(audiotags::Error),
    
    // I/O errors used in general
    Io(std::io::Error),
    
    /// --- Spotify API ---
    // auth_callback_server.rs errors
    // is responsible for setting up a server to receive the OAuth callback
    TimeoutTCP(), // Timeout 2 minutes
    AuthDenied(), // Authentication Denied (User press Denied)

    // reqwest Error used in spotify_api_client.rs
    Reqwest(reqwest::Error),
    UrlParseError(ParseError),
    
    // Spotify API reqwest:
    SpotifyErrorStatus(Status),

    /// --- Audio Backend ---
    // Not initialized backend error
    UnknownAudioBackend(String),
    // Uninitialized librespot backend
    UninitializedLibrespot(),
    // Decoding error with rodio
    RodioDecoderError(DecoderError),
    // Seek error with rodio
    RodioSeekError(SeekError),
    // Librespot general error
    LibrespotError(core::Error),
    // Librespot (Mixer, Backend, Discovery) Errors
    LibrespotInitialization(String),

    /// --- Serde Errors ---
    SerdeJsonError(serde_json::Error),
}

#[derive(Debug)]
pub enum Status {
    BadRequest(String), // 400: The request could not be understood.
    Unauthorized(String), // 401: The request requires user authentication or authorization has been refused for those credentials.
    TooManyRequest(String), // 429: Rate limit of spotify calls.
    Unknown(u16, String), // 403, 500, 502, etc.
}

impl fmt::Display for SpotMpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpotMpError::Error(e) => write!(f, "An error ocurred: {}", e),
            SpotMpError::SQLite(e) => write!(f, "Data Base error: {}", e),
            SpotMpError::AudioTag(e) => write!(f, "Audio tag error: {}", e),
            SpotMpError::Io(e) => write!(f, "I/O error: {}", e),
            SpotMpError::TimeoutTCP() => write!(f, "Timeout in TCP Listener on the authorization callback"),
            SpotMpError::AuthDenied() => write!(f, "Authorization deiend"),
            SpotMpError::Reqwest(e) => write!(f, "Error in Reqwest crate: {}", e),
            SpotMpError::UrlParseError(e) => write!(f, "Error parsing the url: {}", e),
            SpotMpError::SpotifyErrorStatus(Status::BadRequest(t)) => write!(f, "The request could not be understood. Response: {}", t),
            SpotMpError::SpotifyErrorStatus(Status::Unauthorized(t)) => write!(f, "The authentication has expired or you need more credentials. Response: {}", t),
            SpotMpError::SpotifyErrorStatus(Status::TooManyRequest(t)) => write!(f, "API calls rate limit have been surpassed. Response: {}", t),
            SpotMpError::SpotifyErrorStatus(Status::Unknown(n, t)) => write!(f, "Unexpected Spotify API error (Code: {}). Response: {}", n, t),
            SpotMpError::RodioDecoderError(e) => write!(f, "Error in Decoding with rodio: {}", e),
            SpotMpError::RodioSeekError(e) => write!(f, "Error trying to seek the audio track: {}", e),
            SpotMpError::LibrespotError(e) => write!(f, "Unknown error in librespot backend: {}", e),
            SpotMpError::LibrespotInitialization(s) => write!(f, "Failed to initialize Librespot: {}", s),
            SpotMpError::SerdeJsonError(e) => write!(f, "Error trying to serialize or deserialize a file: {}", e),
            SpotMpError::UnknownAudioBackend(s) => write!(f, "Trying to performance an action with audio backend without initialize: {}", s),
            SpotMpError::UninitializedLibrespot() => write!(f, "Tryied to perform an operation over uninitilized librespot backend."),
        }
    }
}

impl std::error::Error for SpotMpError {}

impl From<sqlite::Error> for SpotMpError {
    fn from(error: sqlite::Error) -> Self {
        SpotMpError::SQLite(error)
    }
}

impl From<std::io::Error> for SpotMpError {
    fn from(error: io::Error) -> Self {
        SpotMpError::Io(error)
    }
}

impl From<audiotags::Error> for SpotMpError {
    fn from(error: audiotags::Error) -> Self {
        SpotMpError::AudioTag(error)
    }
}

impl From<reqwest::Error> for SpotMpError {
    fn from(error: reqwest::Error) -> Self {
        SpotMpError::Reqwest(error)
    }
}

impl From<ParseError> for SpotMpError {
    fn from(error: ParseError) -> Self {
        SpotMpError::UrlParseError(error)
    }
}

impl From<DecoderError> for SpotMpError {
    fn from(error: DecoderError) -> Self {
        SpotMpError::RodioDecoderError(error)
    }
}

impl From<SeekError> for SpotMpError {
    fn from(error: SeekError) -> Self {
        SpotMpError::RodioSeekError(error)
    }
}

impl From<core::Error> for SpotMpError {
    fn from(error: core::Error) -> Self {
        SpotMpError::LibrespotError(error)
    }
}

impl From<serde_json::Error> for SpotMpError {
    fn from(error: serde_json::Error) -> Self {
        SpotMpError::SerdeJsonError(error)
    }
}
