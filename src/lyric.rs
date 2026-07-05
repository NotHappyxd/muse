use std::path::Path;
use reqwest::Client;
use serde::Deserialize;
use crate::cache;
use crate::cache::find_cache;

#[derive(Debug, Deserialize)]
struct RawLine {
    start: f64,
    text: String,
    words: Vec<RawWord>,
}

#[derive(Debug, Deserialize)]
struct RawWord {
    word: String,
    start: f64,
    end: f64,
}

#[derive(Debug, Clone)]
pub struct Word {
    pub text: String,
    pub start: u64, // ms
    pub end: u64,   // ms
}


#[derive(Debug, Clone)]
pub struct LyricLine {
    pub timestamp: u64,
    pub line: String,
    pub words: Vec<Word>,
}

#[derive(Debug, Clone)]
pub struct LyricResponse {
    pub song: String,
    pub lyrics: Vec<LyricLine>
}

pub async fn fetch_lyric(title: &str, artists: &Vec<String>, album: &str, length: u32) -> Option<LyricResponse> {
    if title.to_lowercase().contains("a thousand years") && !artists.is_empty() && artists[0].contains("John Michael Howell") { // Test on A Thousand Years
        return parse_forced_alignment_file(Path::new("output.json"), title);
    }

    if let Some(content) = find_cache(title, artists) {
        return Some(LyricResponse {
            song: title.to_string(),
            lyrics: convert_to_timed(&content)
        });
    }

    let client = Client::new();

    let params = [
        ("track_name", title),
        ("artist_name", &artists[0]),
        ("album", if album.is_empty() { title } else { album }),
        ("duration", &length.to_string()),
    ];

    let str = match client
        .get("https://lrclib.net/api/get")
        .query(&params)
        .send()
        .await {
        Ok(res) => {
            res.text().await.unwrap_or(String::from(""))
        },
        Err(_) => String::from(""),
    };

    if str.is_empty() {
        return Some(LyricResponse {
            song: title.to_string(),
            lyrics: vec![]
        })
    }

    let json_data: serde_json::Value = serde_json::from_str(&str).unwrap_or_default();

    if let Some(synced_lyrics) = json_data.get("syncedLyrics").and_then(|v| v.as_str()) {
        let lyrics = convert_to_timed(synced_lyrics);

        cache::write_to_cache(&cache::cache_key(title, artists, album), &synced_lyrics);

        return Some(LyricResponse {
            song: title.to_string(),
            lyrics
        })
    }

    Some(LyricResponse {
        song: title.to_string(),
        lyrics: vec![]
    })
}

pub fn parse_forced_alignment(json_str: &str, song_name: &str) -> Option<LyricResponse> {
    let raw_lines: Vec<RawLine> = match serde_json::from_str(json_str) {
        Ok(raw_lines) => raw_lines,
        Err(_) => return None,
    };

    let lyrics = raw_lines
        .into_iter()
        .map(|raw| LyricLine {
            timestamp: (raw.start * 1000.0).round() as u64,
            line: raw.text,
            words: raw
                .words
                .into_iter()
                .map(|w| Word {
                    text: w.word,
                    start: (w.start * 1000.0).round() as u64,
                    end: (w.end * 1000.0).round() as u64,
                })
                .collect(),
        })
        .collect();

    Some(LyricResponse {
        song: song_name.to_string(),
        lyrics,
    })
}

pub fn parse_forced_alignment_file(
    path: impl AsRef<Path>,
    song_name: &str,
) -> Option<LyricResponse> {
    let json_str = match std::fs::read_to_string(path) {
        Ok(str) => str,
        Err(_) => return None,
    };

    parse_forced_alignment(&json_str, song_name)
}


fn convert_to_timed(str: &str) -> Vec<LyricLine> {
    let mut lyrics: Vec<LyricLine> = Vec::new();

    for lyric in str.lines() {
        if let Some(end) = lyric.find(']') {
            let time = &lyric[1..end];

            let line_text = lyric[end + 1..].trim().to_string();

            if let Some(timestamp) = parse_timestamp(time) {
                lyrics.push(LyricLine {
                    timestamp,
                    line: line_text,
                    words: Vec::new(),
                });
            }
        }
    }

    lyrics
}

fn parse_timestamp(ts: &str) -> Option<u64> {
    let (min, rest) = ts.split_once(':')?;
    let (sec, frac) = rest.split_once('.')?;

    let minutes: u64 = min.parse().ok()?;
    let seconds: u64 = sec.parse().ok()?;

    let millis = match frac.len() {
        1 => frac.parse::<u64>().ok()? * 100,
        2 => frac.parse::<u64>().ok()? * 10,
        3 => frac.parse::<u64>().ok()?,
        _ => return None,
    };

    Some(minutes * 60_000 + seconds * 1_000 + millis)
}