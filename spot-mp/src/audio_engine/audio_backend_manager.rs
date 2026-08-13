use crate::audio_engine::{librespot_backend::LibrespotBackend, rodio_backend::RodioBackend};
use crate::domain::audio_messages::{Messages, Event, ActiveBackend};
use std::sync::mpsc::{Receiver, Sender};

#[allow(dead_code)]
pub struct AudioBackendManager {
    current_backend: Option<ActiveBackend>,
    spotify_backend: LibrespotBackend,
    rodio_backend: RodioBackend,
    receiver: Receiver<Messages>,
    sender: Sender<Event>,
}
