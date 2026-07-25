use std::fs;
use expanduser::expanduser;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(default)]
pub struct Config {
    pub player: String,
    pub header: Header,
    pub color: Color,
    pub lyric_settings: LyricSettings
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(default)]
pub struct Header {
    pub title: String,
    pub centered: bool
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(default)]
pub struct Color {
    pub generate: bool,
    pub k_clusters: u8,
    pub max_color_gen_iterations: u8,
    pub fallback_main: String,
    pub fallback_accent: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(default)]
pub struct LyricSettings {
    pub active_prefix: String,
    pub center: bool
}

impl Default for LyricSettings {
    fn default() -> Self {
        LyricSettings {
            active_prefix: "".to_owned(),
            center: true
        }
    }
}

impl Default for Header {
    fn default() -> Self {
        Header {
            title: "{title} {artists} - {album}".to_owned(),
            centered: false,
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Color {
            generate: true,
            k_clusters: 14,
            max_color_gen_iterations: 30,
            fallback_main: "#afd75f".to_owned(),
            fallback_accent: "#ffffff".to_owned()
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            player: String::from(""),
            header: Header::default(),
            color: Color::default(),
            lyric_settings: LyricSettings::default(),
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

    let config = fs::read(&path)
        .ok()
        .and_then(|b| toml::from_slice(&b).ok());

    match config {
        Some(cfg) => cfg,
        None => {
            let default = Config::default();
            let _ = fs::write(&path, toml::to_string_pretty(&default).unwrap());
            default
        }
    }
}

impl Color {

    pub fn fallback_main_rgb(&self) -> [u8; 3] {
        Self::hex_to_rgb(&self.fallback_main)
            .ok()
            .unwrap_or([175, 215, 95]) // Color::Indexed(149)
    }

    pub fn fallback_accent_rgb(&self) -> [u8; 3] {
        Self::hex_to_rgb(&self.fallback_accent)
            .ok()
            .unwrap_or([95, 95, 0]) // Color::Indexed(58)
    }

    fn hex_to_rgb(hex: &str) -> Result<[u8; 3], &'static str> {
        let hex = hex.trim_start_matches('#');

        if hex.len() != 6 {
            return Err("Hex string must be exactly 6 characters long");
        }

        // Parse red, green, and blue components from hex radix (base 16)
        let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "Invalid hex character in Red component")?;
        let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "Invalid hex character in Green component")?;
        let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "Invalid hex character in Blue component")?;

        Ok([r, g, b])
    }

}