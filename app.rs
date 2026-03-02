use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

use crate::{
    events::{key_to_action, next_event, Action, AppEvent},
    spotify::{PlaybackState, Playlist, SpotifyClient, Track},
    ui,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Panel {
    Playlists,
    Tracks,
    Search,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Searching,
}

pub struct App {
    pub spotify: SpotifyClient,
    pub panel: Panel,
    pub mode: Mode,

    // Playlists
    pub playlists: Vec<Playlist>,
    pub playlist_index: usize,

    // Tracks
    pub tracks: Vec<Track>,
    pub track_index: usize,

    // Search
    pub search_query: String,
    pub search_results: Vec<Track>,
    pub search_index: usize,

    // Playback
    pub playback: Option<PlaybackState>,

    // Status bar
    pub status_msg: Option<String>,

    pub should_quit: bool,
    pub tick_count: u64,
}

impl App {
    pub fn new(spotify: SpotifyClient) -> Self {
        Self {
            spotify,
            panel: Panel::Playlists,
            mode: Mode::Normal,
            playlists: vec![],
            playlist_index: 0,
            tracks: vec![],
            track_index: 0,
            search_query: String::new(),
            search_results: vec![],
            search_index: 0,
            playback: None,
            status_msg: None,
            should_quit: false,
            tick_count: 0,
        }
    }

    pub fn current_tracks(&self) -> &Vec<Track> {
        if self.panel == Panel::Search {
            &self.search_results
        } else {
            &self.tracks
        }
    }

    pub fn current_track_index(&self) -> usize {
        if self.panel == Panel::Search {
            self.search_index
        } else {
            self.track_index
        }
    }

    fn set_status(&mut self, msg: impl Into<String>) {
        self.status_msg = Some(msg.into());
    }

    async fn load_playlists(&mut self) {
        match self.spotify.get_playlists(50, 0).await {
            Ok(p) => {
                self.playlists = p;
                self.playlist_index = 0;
                if !self.playlists.is_empty() {
                    self.load_playlist_tracks().await;
                }
            }
            Err(e) => self.set_status(format!("Error loading playlists: {}", e)),
        }
    }

    async fn load_playlist_tracks(&mut self) {
        if let Some(pl) = self.playlists.get(self.playlist_index) {
            let id = pl.id.clone();
            match self.spotify.get_playlist_tracks(&id).await {
                Ok(tracks) => {
                    self.tracks = tracks;
                    self.track_index = 0;
                }
                Err(e) => self.set_status(format!("Error loading tracks: {}", e)),
            }
        }
    }

    async fn refresh_playback(&mut self) {
        match self.spotify.get_playback().await {
            Ok(state) => self.playback = state,
            Err(_) => {}
        }
    }

    async fn handle_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,

            Action::Tab => {
                self.panel = match self.panel {
                    Panel::Playlists => Panel::Tracks,
                    Panel::Tracks => Panel::Search,
                    Panel::Search => Panel::Playlists,
                };
            }

            Action::NavUp => match self.panel {
                Panel::Playlists => {
                    if self.playlist_index > 0 {
                        self.playlist_index -= 1;
                        self.load_playlist_tracks().await;
                    }
                }
                Panel::Tracks => {
                    if self.track_index > 0 {
                        self.track_index -= 1;
                    }
                }
                Panel::Search => {
                    if self.search_index > 0 {
                        self.search_index -= 1;
                    }
                }
            },

            Action::NavDown => match self.panel {
                Panel::Playlists => {
                    if self.playlist_index + 1 < self.playlists.len() {
                        self.playlist_index += 1;
                        self.load_playlist_tracks().await;
                    }
                }
                Panel::Tracks => {
                    if self.track_index + 1 < self.tracks.len() {
                        self.track_index += 1;
                    }
                }
                Panel::Search => {
                    if self.search_index + 1 < self.search_results.len() {
                        self.search_index += 1;
                    }
                }
            },

            Action::NavLeft => {
                if self.panel == Panel::Tracks || self.panel == Panel::Search {
                    self.panel = Panel::Playlists;
                }
            }

            Action::NavRight => {
                if self.panel == Panel::Playlists {
                    self.panel = Panel::Tracks;
                }
            }

            Action::Select => {
                let tracks = if self.panel == Panel::Search {
                    self.search_results.clone()
                } else {
                    self.tracks.clone()
                };
                let idx = if self.panel == Panel::Search {
                    self.search_index
                } else {
                    self.track_index
                };

                if let Some(track) = tracks.get(idx) {
                    let uri = track.uri.clone();
                    let name = track.name.clone();
                    match self.spotify.play(Some(&uri), None).await {
                        Ok(_) => {
                            self.set_status(format!("▶ Playing: {}", name));
                            self.refresh_playback().await;
                        }
                        Err(e) => self.set_status(format!("Error: {}", e)),
                    }
                }
            }

            Action::TogglePlay => {
                if let Some(ref pb) = self.playback.clone() {
                    if pb.is_playing {
                        match self.spotify.pause().await {
                            Ok(_) => self.set_status("⏸ Paused".into()),
                            Err(e) => self.set_status(format!("Error: {}", e)),
                        }
                    } else {
                        match self.spotify.play(None, None).await {
                            Ok(_) => self.set_status("▶ Resumed".into()),
                            Err(e) => self.set_status(format!("Error: {}", e)),
                        }
                    }
                    self.refresh_playback().await;
                }
            }

            Action::Next => {
                match self.spotify.next().await {
                    Ok(_) => self.set_status("⏭ Next track".into()),
                    Err(e) => self.set_status(format!("Error: {}", e)),
                }
                tokio::time::sleep(Duration::from_millis(300)).await;
                self.refresh_playback().await;
            }

            Action::Previous => {
                match self.spotify.previous().await {
                    Ok(_) => self.set_status("⏮ Previous track".into()),
                    Err(e) => self.set_status(format!("Error: {}", e)),
                }
                tokio::time::sleep(Duration::from_millis(300)).await;
                self.refresh_playback().await;
            }

            Action::VolumeUp => {
                if let Some(ref pb) = self.playback.clone() {
                    if let Some(ref dev) = pb.device {
                        let new_vol = (dev.volume_percent.unwrap_or(50) as u16 + 5).min(100) as u8;
                        match self.spotify.set_volume(new_vol).await {
                            Ok(_) => self.set_status(format!("🔊 Volume: {}%", new_vol)),
                            Err(e) => self.set_status(format!("Error: {}", e)),
                        }
                        self.refresh_playback().await;
                    }
                }
            }

            Action::VolumeDown => {
                if let Some(ref pb) = self.playback.clone() {
                    if let Some(ref dev) = pb.device {
                        let vol = dev.volume_percent.unwrap_or(50) as i16;
                        let new_vol = (vol - 5).max(0) as u8;
                        match self.spotify.set_volume(new_vol).await {
                            Ok(_) => self.set_status(format!("🔉 Volume: {}%", new_vol)),
                            Err(e) => self.set_status(format!("Error: {}", e)),
                        }
                        self.refresh_playback().await;
                    }
                }
            }

            Action::ToggleShuffle => {
                if let Some(ref pb) = self.playback.clone() {
                    let new_state = !pb.shuffle_state;
                    match self.spotify.set_shuffle(new_state).await {
                        Ok(_) => self.set_status(format!(
                            "Shuffle: {}",
                            if new_state { "ON ⇌" } else { "OFF" }
                        )),
                        Err(e) => self.set_status(format!("Error: {}", e)),
                    }
                    self.refresh_playback().await;
                }
            }

            Action::ToggleRepeat => {
                if let Some(ref pb) = self.playback.clone() {
                    let new_state = match pb.repeat_state.as_str() {
                        "off" => "context",
                        "context" => "track",
                        _ => "off",
                    };
                    match self.spotify.set_repeat(new_state).await {
                        Ok(_) => self.set_status(format!("Repeat: {}", new_state)),
                        Err(e) => self.set_status(format!("Error: {}", e)),
                    }
                    self.refresh_playback().await;
                }
            }

            Action::Search => {
                self.panel = Panel::Search;
                self.mode = Mode::Searching;
            }

            Action::Escape => {
                self.mode = Mode::Normal;
            }

            Action::Char(c) => {
                if self.mode == Mode::Searching {
                    self.search_query.push(c);
                    if self.search_query.len() >= 2 {
                        let q = self.search_query.clone();
                        match self.spotify.search(&q).await {
                            Ok(results) => {
                                self.search_results = results;
                                self.search_index = 0;
                            }
                            Err(e) => self.set_status(format!("Search error: {}", e)),
                        }
                    }
                }
            }

            Action::Backspace => {
                if self.mode == Mode::Searching {
                    self.search_query.pop();
                    if self.search_query.is_empty() {
                        self.search_results.clear();
                    } else {
                        let q = self.search_query.clone();
                        match self.spotify.search(&q).await {
                            Ok(results) => {
                                self.search_results = results;
                                self.search_index = 0;
                            }
                            Err(_) => {}
                        }
                    }
                }
            }

            Action::Refresh => {
                self.load_playlists().await;
                self.refresh_playback().await;
                self.set_status("✓ Refreshed".into());
            }
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        // Initial data load
        self.load_playlists().await;
        self.refresh_playback().await;

        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let tick_rate = Duration::from_millis(250);
        let playback_poll = 3000u64; // ms between playback refreshes

        loop {
            terminal.draw(|f| ui::render(f, self))?;

            if let Some(event) = next_event(tick_rate)? {
                match event {
                    AppEvent::Key(key) => {
                        if let Some(action) = key_to_action(&key) {
                            self.handle_action(action).await;
                        }
                    }
                    AppEvent::Tick => {
                        self.tick_count += 1;
                        // Refresh playback every ~3 seconds
                        if self.tick_count % (playback_poll / 250) == 0 {
                            self.refresh_playback().await;
                        }
                        // Clear status after ~4 seconds
                        if self.tick_count % 16 == 0 {
                            self.status_msg = None;
                        }
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }

        // Restore terminal
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        Ok(())
    }
}
