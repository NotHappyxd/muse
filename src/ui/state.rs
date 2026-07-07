use std::cell::Cell;
use std::time::Instant;
use tokio::sync::mpsc::UnboundedSender;
use crate::config::Config;
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
    pub anchor_position: u128,
    pub anchor_instant: Option<Instant>,
    pub is_playing: bool,
    pub quit: bool,
    pub manual_scroll_offset: Cell<i16>,
    pub tx: Option<UnboundedSender<AppEvent>>,
    pub config: Config
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
    pub fn new(config: Config) -> Self {
        let theme = if config.color.generate {
            [0, 0, 0]
        }else {
            config.color.fallback_main_rgb()
        };

        let accent = if config.color.generate {
            [0, 0, 0]
        }else {
            config.color.fallback_accent_rgb()
        };

        Self {
            active_song: None,
            lyrics: None,
            song_theme: theme,
            song_accent: accent,
            anchor_position: 0,
            anchor_instant: None,
            is_playing: false,
            quit: false,
            manual_scroll_offset: Cell::new(0),
            tx: None,
            config,
        }
    }

    pub fn set_progress(&mut self, position: u128, is_playing: bool, at: Instant) {
        self.anchor_position = position;
        self.anchor_instant = if is_playing { Some(at) } else { None };
        self.is_playing = is_playing;
    }

    pub fn current_progress(&self) -> u128 {
        match self.anchor_instant {
            None => self.anchor_position,
            Some(instant) => {
                self.anchor_position + Instant::now().saturating_duration_since(instant).as_millis()
            }
        }
    }
}