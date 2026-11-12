use log::error;
use rand::RngExt;
use std::collections::VecDeque;
use tokio::sync::mpsc;

use crate::domain::auth_models::LibrespotData;
use crate::domain::{
    audio_messages::{Event, Messages},
    playback_flow::PlaybackFlow,
    song::Song,
};
use crate::services::audio_engine::audio_backend_manager::AudioBackendManager;
use crate::state_store::app::action::{Action, MusicPlayerAction};

pub struct MusicPlayerController {
    reciever: mpsc::UnboundedReceiver<Event>,
    sender: mpsc::UnboundedSender<Messages>,
    now_playing: Option<Song>,
    user_queue: VecDeque<Song>,
    playlist_queue: VecDeque<Song>,
    memory: VecDeque<Song>,
    playback_flow: PlaybackFlow,
}

impl Default for MusicPlayerController {
    fn default() -> Self {
        Self::new()
    }
}

impl MusicPlayerController {
    // --- Constructor ---
    pub fn new() -> Self {
        let (m_sender, m_reciever): (
            mpsc::UnboundedSender<Messages>,
            mpsc::UnboundedReceiver<Messages>,
        ) = mpsc::unbounded_channel();
        let (e_sender, e_reciever): (
            mpsc::UnboundedSender<Event>,
            mpsc::UnboundedReceiver<Event>,
        ) = mpsc::unbounded_channel();

        // Initialize and Start the audio engine
        let audio_engine = AudioBackendManager::new(m_reciever, e_sender);
        audio_engine.start_engine();

        Self {
            reciever: e_reciever,
            sender: m_sender,
            now_playing: None,
            user_queue: VecDeque::new(),
            playlist_queue: VecDeque::new(),
            memory: VecDeque::new(),
            playback_flow: PlaybackFlow::Normal,
        }
    }

    fn process_event(&mut self, event: Event) -> Action {
        match event {
            Event::Finished => {
                // 1. User-added queue has highest priority
                if let Some(next_song) = self.user_queue.pop_front() {
                    return self.play(next_song);
                }

                // 2. If playback flow is InCycle, repeat the current song without pushing to memory
                if matches!(self.playback_flow, PlaybackFlow::InCycle) {
                    if let Some(current_song) = self.now_playing.clone() {
                        return self.play_internal(current_song, false);
                    }
                    return Action::NoOp;
                }

                // 3. Otherwise advance through the playlist queue
                if !self.playlist_queue.is_empty() {
                    match self.playback_flow {
                        PlaybackFlow::Normal => {
                            let next_song = self.playlist_queue.pop_front().unwrap();
                            self.play(next_song)
                        }
                        PlaybackFlow::Random => self.play_random(),
                        PlaybackFlow::InCycle => Action::NoOp,
                    }
                } else {
                    Action::MusicPlayer(MusicPlayerAction::SongFinished)
                }
            }
            Event::SpotifyInitialized(credentials) => {
                // This retrieves the credentials for storing them in ControllerHub class
                Action::MusicPlayer(MusicPlayerAction::LibrespotInitialized(credentials))
            }
            Event::Error(e) => {
                error!("{}", e);
                Action::UnexpectedErrorHappened(e.to_string())
            }
        }
    }

    // Async event receiver for select! loop in ControllerHub
    pub async fn recv_event(&mut self) -> Action {
        match self.reciever.recv().await {
            Some(event) => self.process_event(event),
            None => Action::NoOp,
        }
    }

    // Event handler for Audio Engine responses (non-blocking)
    // This function returns an Action for updating the frontend
    pub fn handle_event(&mut self) -> Action {
        match self.reciever.try_recv() {
            Ok(event) => self.process_event(event),
            Err(mpsc::error::TryRecvError::Disconnected) => {
                error!("The connection between MusicPlayerController and AudioBackendManager disappeared!");
                Action::NoOp
            }
            Err(mpsc::error::TryRecvError::Empty) => Action::NoOp,
        }
    }

    // This function starts the librespot login and logout flow with a message; the result is
    // caught with handle_event / recv_event.

    pub fn login(&self, client_id: &str, data: Option<LibrespotData>) {
        if let Err(e) = self
            .sender
            .send(Messages::LoginLibrespot(client_id.to_string(), data))
        {
            error!("{}", e);
        }
    }
    pub fn logout(&self) {
        if let Err(e) = self.sender.send(Messages::LogoutLibrespot) {
            error!("{}", e);
        }
    }

    // ---
    // The next functions don't retrieve an Action because they are safe and the app can update
    // directly the store without confirmation
    // ---

    // Change reproduction flow function
    pub fn set_flow(&mut self, new_flow: &PlaybackFlow) {
        self.playback_flow = new_flow.to_owned();
    }

    // Add song to high-priority user queue
    pub fn add_to_queue(&mut self, song: Song) {
        self.user_queue.push_back(song);
    }

    // Erase from both queues
    pub fn erase_from_queue(&mut self, song_id: &str) {
        self.user_queue.retain(|x| x.song_data() != song_id);
        self.playlist_queue.retain(|x| x.song_data() != song_id);
    }

    // Erase the entire queue
    pub fn clear_queue(&mut self) {
        self.user_queue.clear();
        self.playlist_queue.clear();
    }

    // ---
    // Reproductor Functions:
    fn play_internal(&mut self, song: Song, record_history: bool) -> Action {
        if let Err(e) = self.sender.send(Messages::Play(song.to_source())) {
            error!("{}", e);
        }

        // Adding the current playing song to memory (for Previous func)
        if record_history
            && let Some(playing_song) = self.now_playing.take()
        {
            if self.memory.len() >= 5 {
                self.memory.pop_back();
            }
            self.memory.push_front(playing_song);
        }

        // Saves the current playing song value
        self.now_playing = Some(song.clone());
        Action::MusicPlayer(MusicPlayerAction::StartPlayingSong(song))
    }

    pub fn play(&mut self, song: Song) -> Action {
        self.play_internal(song, true)
    }

    fn play_random(&mut self) -> Action {
        if self.playlist_queue.is_empty() {
            return Action::NoOp;
        }
        let rand_num = rand::rng().random_range(0..self.playlist_queue.len());
        if let Some(next_song) = self.playlist_queue.remove(rand_num) {
            self.play(next_song)
        } else {
            Action::NoOp
        }
    }

    pub fn play_playlist(&mut self, playlist: Vec<Song>) -> Action {
        if playlist.is_empty() {
            return Action::NoOp;
        }

        // Replace only the playlist queue so manually queued songs are preserved
        self.playlist_queue.clear();

        let mut iter = playlist.into_iter();
        let play_now = iter.next().unwrap();
        for song in iter {
            self.playlist_queue.push_back(song);
        }
        self.play(play_now)
    }

    pub fn toggle_pause(&self) {
        if let Err(e) = self.sender.send(Messages::TogglePause) {
            error!("{}", e);
        }
    }

    pub fn seek(&self, position: u32) {
        if let Err(e) = self.sender.send(Messages::Seek(position)) {
            error!("{}", e);
        }
    }

    pub fn skip_to_next(&mut self) -> Action {
        if let Some(next_song) = self.user_queue.pop_front() {
            return self.play(next_song);
        }

        if self.playlist_queue.is_empty() {
            Action::NoOp
        } else {
            match self.playback_flow {
                PlaybackFlow::Random => self.play_random(),
                _ => {
                    let next_song = self.playlist_queue.pop_front().unwrap();
                    self.play(next_song)
                }
            }
        }
    }

    pub fn previous(&mut self) -> Action {
        if let Some(prev_song) = self.memory.pop_front() {
            // Put the currently playing song back at the front of the playlist queue
            // and do not push it into memory again (avoids A <-> B infinite loop)
            if let Some(current_song) = self.now_playing.take() {
                self.playlist_queue.push_front(current_song);
            }
            self.play_internal(prev_song, false)
        } else {
            Action::NoOp
        }
    }
}
