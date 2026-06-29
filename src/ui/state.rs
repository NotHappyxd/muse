use std::cell::Cell;
use tokio::sync::mpsc::UnboundedSender;
use crate::lyric::LyricResponse;
use crate::watcher::AppEvent;

#[derive(Debug, Clone)]
pub struct Song {
    pub title: String,
    pub album: String,
    pub artists: Vec<String>,
    pub length: u32,
}

#[derive(Debug, Default)]
pub struct App {
    pub active_song: Option<Song>,
    pub lyrics: Option<LyricResponse>,
    pub song_theme: [u8; 3],
    pub song_accent: [u8; 3],
    pub progress: u128,
    pub quit: bool,
    pub manual_scroll_offset: Cell<i16>,
    pub tx: Option<UnboundedSender<AppEvent>>,
}


impl Song {
    pub fn new(title: String, album: String, artists: Vec<String>, length: u32) -> Self {
        Self {
            title,
            album,
            artists,
            length,
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_progress(&mut self, progress: u128) {
        self.progress = progress;
    }
}
