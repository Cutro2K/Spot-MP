use tokio::sync::mpsc;
use std::collections::VecDeque;
use log::error;
use rand::RngExt;

use crate::app::action::{Action, MusicPlayerAction};
use crate::domain::auth_models::LibrespotData;
use crate::domain::{audio_messages::{Event, Messages}, song::Song, playback_flow::PlaybackFlow};
use crate::audio_engine::audio_backend_manager::AudioBackendManager;

pub struct MusicPlayerController {
    reciever: mpsc::UnboundedReceiver<Event>,
    sender: mpsc::UnboundedSender<Messages>,
    now_playing: Option<Song>,
    queue: VecDeque<Song>,
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
        let (m_sender, m_reciever): (mpsc::UnboundedSender<Messages>, mpsc::UnboundedReceiver<Messages>) = mpsc::unbounded_channel();
        let (e_sender, e_reciever): (mpsc::UnboundedSender<Event>, mpsc::UnboundedReceiver<Event>) = mpsc::unbounded_channel();

        // Initialize and Start the audio engine
        let audio_engine = AudioBackendManager::new(m_reciever, e_sender);
        audio_engine.start_engine();
        
        Self {
            reciever: e_reciever,
            sender: m_sender,
            now_playing: None,
            queue: VecDeque::new(),
            memory: VecDeque::new(),
            playback_flow: PlaybackFlow::Normal,
        }
    }

    // Event handler for Audio Engine responses
    // This functions returns an Action for update the frontend
    pub fn handle_event(&mut self) -> Action {
        match self.reciever.try_recv() {
            Ok(event) => {
                match event {
                    Event::Finished => {
                        if !self.queue.is_empty() {
                            match self.playback_flow {
                                PlaybackFlow::Normal => {
                                    // Extract the new song from queue
                                    let next_song = self.queue.pop_front().unwrap();
                            
                                    self.play(next_song)
                                },
                                PlaybackFlow::Random => {
                                   self.play_random()
                                },
                                PlaybackFlow::InCycle => {
                                    if let Some(current_song) = &self.now_playing {
                                        self.play(current_song.clone())
                                    } else {
                                        Action::NoOp 
                                    }
                                }
                            }
                        }
                        else {
                            Action::MusicPlayer(MusicPlayerAction::SongFinished)
                        }
                    },
                    Event::SpotifyInitilized(credentials) => {
                        // This retrieves the credentials for storing them in ControllerHub class
                        Action::MusicPlayer(MusicPlayerAction::LibrespotInitialized(credentials)) 
                    },
                    Event::Error(e) => {
                        error!("{}", e);
                        Action::UnexpectedErrorHappened(e.to_string())
                    }
                }
            },
            Err(mpsc::error::TryRecvError::Disconnected) => {
                panic!("The connection between MusicPlayerController and AudioBackendManager dissapear!");
            },
            Err(mpsc::error::TryRecvError::Empty) => {
                Action::NoOp
            },
        }
    }
    
    // This functions starts the librespot login and logout flow with a message the result is
    // catched with Handle Event function.
    
    pub fn login(&self, client_id: &str, data: Option<LibrespotData>) {
        if let Err(e) = self.sender.send(Messages::LoginLibrespot(client_id.to_string(), data)) {
            error!("{}", e);
        }
    }
    pub fn logout(&self) {
        if let Err(e) = self.sender.send(Messages::LogoutLibrespot) {
            error!("{}", e);
        }
    }
    
    // ---
    // The next functions doesn't retrieve an Action because they are safety and I can update
    // directly the store without confirmation
    // ---

    // Change reproduction flow function
    pub fn set_flow(&mut self, new_flow: &PlaybackFlow) {
        self.playback_flow = new_flow.to_owned();
    }
        
    // Add song to queue
    pub fn add_to_queue(&mut self, song: Song) {
        self.queue.push_back(song);
    }

    // Erase from queue
    pub fn erase_from_queue(&mut self, song_id: &str) {
        self.queue.retain(|x| x.song_data() != song_id);
    }
    
    // Erase the entire queue
    pub fn clear_queue(&mut self) {
        // Using Vec::new() for clearing the ram memory
        self.queue = VecDeque::new();
    }

    // ---
    // Reproductor Functions: 
    pub fn play(&mut self, song: Song) -> Action {
        // Plays the new song
        //
        if let Err(e) = self.sender.send(Messages::Play(song.to_source())) {
            error!("{}", e);
        }
        
        // Adding the current playing song to memory (for Previous func)
        if let Some(playing_song) = self.now_playing.take() {
            if self.memory.len() >= 5 {
                self.memory.pop_back();
            }
            self.memory.push_front(playing_song);
        }

        // Saves the current playing song value
        self.now_playing = Some(song.clone());
        Action::MusicPlayer(MusicPlayerAction::StartPlayingSong(song))
    }
    
    fn play_random(&mut self) -> Action {
        let rand_num = rand::rng().random_range(0..self.queue.len());
        let next_song = self.queue[rand_num].clone();
        self.queue.remove(rand_num);
        self.play(next_song)
    }

    pub fn play_playlist(&mut self, playlist: Vec<Song>) -> Action {
        if playlist.is_empty() {
            return Action::NoOp;
        }

        let play_now = playlist[0].clone();
        let mut first = true;
        for song in playlist {
            if first { first = false; }
            else {
                self.add_to_queue(song);
            }
        }
        self.play(play_now)
    }

    pub fn toggle_pause(&self) {
        if let Err(e) = self.sender.send(Messages::TogglePause) {
            error!("{}", e);
        }
    }

    pub fn skip_to_next(&mut self) -> Action {
        if self.queue.is_empty() {
            Action::NoOp
        }
        else {
            match self.playback_flow {
                PlaybackFlow::Random => {
                    self.play_random()
                },
                _ => {
                    let next_song = self.queue.pop_front().unwrap();
                    self.play(next_song)
                }
            }
        }
    }

    pub fn previous(&mut self) -> Action {
        if let Some(next_song) = self.memory.pop_front() {
            self.play(next_song)
        }
        else {
            Action::NoOp
        }
    }
}
