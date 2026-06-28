use std::cell::Cell;
use crate::lyric::LyricResponse;

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
    pub progress: u128,
    pub quit: bool,
    pub manual_scroll_offset: Cell<i16>,
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
