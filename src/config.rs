use std::fs;
use expanduser::expanduser;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub header: String,
    pub header_centered: bool,
    pub k_clusters: u8,
    pub max_color_gen_iterations: u8,
    pub player: String,
    pub active_lyric_prefix: String,
    pub center_lyrics: bool
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PartialConfig {
    pub header: Option<String>,
    pub header_centered: Option<bool>,
    pub k_clusters: Option<u8>,
    pub max_color_gen_iterations: Option<u8>,
    pub player: Option<String>,
    pub active_lyric_prefix: Option<String>,
    pub center_lyrics: Option<bool>
}

impl Default for Config {
    fn default() -> Self {
        Config {
            header: String::from("{title} {artists} - {album}"),
            header_centered: false,
            k_clusters: 12,
            max_color_gen_iterations: 30,
            player: String::from(""),
            active_lyric_prefix: String::from(""),
            center_lyrics: true
        }
    }
}

impl Config {
    fn merge(self, partial: PartialConfig) -> Self {
        Self {
            header: partial.header.unwrap_or(self.header),
            header_centered: partial.header_centered.unwrap_or(self.header_centered),
            k_clusters: partial.k_clusters.unwrap_or(self.k_clusters),
            max_color_gen_iterations: partial.max_color_gen_iterations.unwrap_or(self.max_color_gen_iterations),
            player: partial.player.unwrap_or(self.player),
            active_lyric_prefix: partial.active_lyric_prefix.unwrap_or(self.active_lyric_prefix),
            center_lyrics: partial.center_lyrics.unwrap_or(self.center_lyrics)
        }
    }
}

pub fn init() -> Config {
    let mut path = match expanduser("~/.config/muse") {
        Ok(p) => p,
        Err(_) => return Config::default(),
    };

    if let Err(e) = fs::create_dir_all(&path) {
        eprintln!("Failed to create dirs: {e}");
        return Config::default();
    }

    path.push("config.toml");
    let defaults = Config::default();

    let partial: PartialConfig = match fs::read(&path) {
        Ok(bytes) => match toml::from_slice(&bytes) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("Invalid config: {e}");
                return defaults;
            }
        },
        Err(_) => {
            let _ = fs::write(&path, toml::to_string_pretty(&defaults).unwrap());
            return defaults;
        }
    };

    defaults.merge(partial)
}