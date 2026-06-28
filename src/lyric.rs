use reqwest::Client;
use crate::cache;
use crate::cache::find_cache;
use crate::ui::state::Song;

#[derive(Debug, Clone)]
pub struct LyricLine {
    pub timestamp: u64,
    pub line: String
}
#[derive(Debug, Clone)]
pub struct LyricResponse {
    pub song: String,
    pub lyrics: Vec<LyricLine>
}

pub async fn fetch_lyric(song: &Song) -> Option<LyricResponse> {
    if let Some(content) = find_cache(&song.title, &song.artists) {
        return Some(LyricResponse {
            song: song.title.to_string(),
            lyrics: convert_to_timed(&content)
        });
    }
    let client = Client::new();

    let params = [
        ("track_name", &song.title),
        ("artist_name", &song.artists[0]),
        ("album", if song.album.is_empty() { &song.title } else { &song.album }),
        ("duration", &song.length.to_string()),
    ];

    let str = match client
        .get("https://lrclib.net/api/get")
        .query(&params)
        .send()
        .await {
        Ok(res) => res.text().await.unwrap_or("".to_string()),
        Err(_) => "".to_string(),
    };

    if str.is_empty() {
        return Some(LyricResponse {
            song: song.title.clone(),
            lyrics: vec![]
        })
    }


    let json_data: serde_json::Value = serde_json::from_str(&str).unwrap_or_default();

    if let Some(synced_lyrics) = json_data.get("syncedLyrics").and_then(|v| v.as_str()) {
        let lyrics = convert_to_timed(synced_lyrics);

        cache::write_to_cache(&cache::cache_key(&song.title, &song.artists, &song.album), &synced_lyrics);

        return Some(LyricResponse {
            song: song.title.to_string(),
            lyrics
        })
    }

    Some(LyricResponse {
        song: song.title.to_string(),
        lyrics: vec![]
    })
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
                    line: line_text
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