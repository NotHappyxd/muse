use crate::colorgen::generator::Theme;
use crate::config;
use crate::config::Config;
use crate::lyric::{fetch_lyric, LyricResponse};
use crate::theme::fetch_theme;
use mpris::{Metadata, PlaybackStatus, Player, PlayerFinder};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::watch::Receiver;

#[derive(Debug)]
pub enum AppEvent {
    PlayerDetached,
    Idle,
    SongChanged {
        title: String,
        album: String,
        artists: Vec<String>,
        length: u32,
    },
    PlaybackAnchor {
        position_ms: u128,
        is_playing: bool,
        at: Instant,
    },
    LyricsFetched {
        lyrics: Option<LyricResponse>,
    },
    Error {
        error: String,
    },
    PlayerCommand(PlayerCommand),
    ThemeFetched {
        song_title: String,
        theme: Theme,
    },
}

#[derive(Debug)]
pub enum PlayerCommand {
    Pause,
    Next,
    Previous,
    Skip(u64)
}
const POLL_MS: u64 = 500;
const RETRY_COUNT: u8 = 3;

#[derive(Default)]
struct WatcherState {
    current_title: Option<String>,
    current_bus: Option<String>,
    current_album: Option<String>,
    album_retry: u8,
    anchor_position: Option<u128>,
    anchor_instant: Option<Instant>,
    anchor_playing: bool,
    last_real_position: Option<u128>,
}

struct SongInfo {
    title: String,
    album: String,
    artists: Vec<String>,
    length: u32,
    art_url: Option<String>,
}

pub async fn run_watcher(
    tx: UnboundedSender<AppEvent>,
    mut shutdown_rx: Receiver<bool>,
    config: &Config,
) {
    let mut state = WatcherState::default();

    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    break;
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(POLL_MS)) => {
                poll(&tx, &mut state, config);
            }
        }
    }
}

fn poll(tx: &UnboundedSender<AppEvent>, state: &mut WatcherState, config: &Config) {
    let finder = match PlayerFinder::new() {
        Ok(f) => f,
        Err(e) => {
            let _ = tx.send(AppEvent::Error {
                error: format!("D-Bus connection failed: {e}"),
            });
            return;
        }
    };

    let players = match finder.iter_players() {
        Ok(players) => players,
        Err(_) => return,
    };

    let players_vec: Vec<_> = players
        .filter_map(Result::ok)
        .filter(|player| player.bus_name().contains(&config.player))
        .collect();

    let active = if players_vec.len() <= 1 {
        players_vec.into_iter().next()
    } else {
        players_vec.into_iter().find(|player| {
            let playing = player
                .get_playback_status()
                .ok()
                .map(|playback| playback == PlaybackStatus::Playing)
                .unwrap_or(false);

            playing
        })
    };

    match active {
        None => handle_no_player(state, tx),
        Some(player) => handle_active_player(tx, state, &player, config),
    }
}

fn handle_no_player(state: &mut WatcherState, tx: &UnboundedSender<AppEvent>) {
    if state.current_bus.is_some() {
        state.reset_session();
        let _ = tx.send(AppEvent::PlayerDetached);
    }

    if state.anchor_instant.is_some() || state.anchor_playing {
        let now = Instant::now();
        let estimated_position = state.predicted_position(now);
        state.pause_anchor(estimated_position);

        let _ = tx.send(AppEvent::PlaybackAnchor {
            position_ms: estimated_position,
            at: now,
            is_playing: false,
        });
    }

    let _ = tx.send(AppEvent::Idle);
}

fn handle_active_player(
    tx: &UnboundedSender<AppEvent>,
    state: &mut WatcherState,
    player: &Player,
    config: &Config,
) {
    let bus = player.bus_name().to_owned();

    if state.current_bus.as_deref() != Some(&bus) {
        if state.current_bus.is_some() {
            let _ = tx.send(AppEvent::PlayerDetached);
        }

        state.reset_session();
        state.current_bus = Some(bus);
    }

    let paused_before_sync = !state.anchor_playing;
    sync_playback_drift(player, state, tx);
    let was_paused = state.anchor_playing && paused_before_sync;

    match player.get_metadata() {
        Err(e) => {
            let _ = tx.send(AppEvent::Error {
                error: format!("Failed to get metadata: {e}"),
            });
        }

        Ok(metadata) => handle_metadata(&metadata, state, config, tx, was_paused),
    }
}

fn sync_playback_drift(player: &Player, state: &mut WatcherState, tx: &UnboundedSender<AppEvent>) {
    let now = Instant::now();
    let paused = matches!(player.get_playback_status(), Ok(PlaybackStatus::Paused));
    let Ok(pos) = player.get_position() else { return };
    let real_ms = pos.as_millis();

    if paused {
        state.pause_anchor(real_ms);
        state.last_real_position = None;

        let _ = tx.send(AppEvent::PlaybackAnchor {
            position_ms: real_ms,
            is_playing: false,
            at: now,
        });
        return;
    }

    let is_new_reading = state.last_real_position != Some(real_ms);

    if !state.anchor_playing || is_new_reading {
        state.update_anchor(real_ms, now, true);
        state.last_real_position = Some(real_ms);

        let _ = tx.send(AppEvent::PlaybackAnchor {
            position_ms: real_ms,
            is_playing: true,
            at: now,
        });
    }
}

fn handle_metadata(
    metadata: &Metadata,
    state: &mut WatcherState,
    config: &Config,
    tx: &UnboundedSender<AppEvent>,
    was_paused: bool,
) {
    let song_info = SongInfo::from_metadata(metadata);

    let same_title = state.current_title.as_ref() == Some(&song_info.title);

    if same_title {
        if was_paused
            && config.color.generate
            && let Some(art_url) = song_info.art_url
        {
            fetch_theme_task(song_info.title.clone(), art_url, &config.color, tx);
        }
        return;
    }

    let valid_album = !song_info.album.is_empty();
    let passed_retry_threshold = state.album_retry >= RETRY_COUNT;

    if !valid_album && !passed_retry_threshold {
        state.album_retry += 1;
        return;
    }

    state.current_title = Some(song_info.title.clone());
    state.current_album = Some(song_info.album.clone());
    state.album_retry = 0;

    let artists = song_info.artists.clone();

    let _ = tx.send(AppEvent::SongChanged {
        title: song_info.title.clone(),
        album: song_info.album.clone(),
        artists,
        length: song_info.length,
    });

    if let Some(art_url) = song_info.art_url {
        let tx2 = tx.clone();

        let generate_theme = config.color.generate;

        if generate_theme {
            fetch_theme_task(song_info.title.clone(), art_url, &config.color, tx)
        }

        tokio::spawn(async move {
            let lyrics = fetch_lyric(
                &song_info.title,
                &song_info.artists,
                &song_info.album,
                song_info.length,
            )
            .await;
            let _ = tx2.send(AppEvent::LyricsFetched { lyrics });
        });
    }
}

fn fetch_theme_task(
    song_title: String,
    art_url: String,
    color: &config::Color,
    tx: &UnboundedSender<AppEvent>,
) {
    let channel = tx.clone();
    let k_clusters = color.k_clusters;
    let max_iterations = color.max_color_gen_iterations;
    let min_chroma = color.min_chroma;
    let min_chroma_percentage = color.min_chroma_percentage;

    tokio::spawn(async move {
        fetch_theme(
            song_title,
            art_url,
            &channel,
            k_clusters,
            max_iterations,
            min_chroma,
            min_chroma_percentage
        )
        .await
    });
}

impl WatcherState {
    fn predicted_position(&self, now: Instant) -> u128 {
        let Some(anchor_position) = self.anchor_position else {
            return 0;
        };

        if let (true, Some(anchor_at)) = (self.anchor_playing, self.anchor_instant) {
            return anchor_position + now.saturating_duration_since(anchor_at).as_millis();
        }

        anchor_position
    }

    fn reset_session(&mut self) {
        self.current_bus = None;
        self.current_title = None;
        self.current_album = None;
    }

    fn update_anchor(&mut self, position: u128, now: Instant, is_playing: bool) {
        self.anchor_position = Some(position);
        self.anchor_instant = Some(now);
        self.anchor_playing = is_playing;
    }

    fn pause_anchor(&mut self, position: u128) {
        self.anchor_position = Some(position);
        self.anchor_instant = None;
        self.anchor_playing = false;
    }
}

impl SongInfo {
    fn from_metadata(metadata: &Metadata) -> Self {
        let title = metadata.title().unwrap_or("Unknown").to_owned();
        let album = metadata
            .album_name()
            .filter(|a| !a.trim().is_empty() && *a != "Unknown")
            .map(|a| a.to_owned())
            .unwrap_or_default();
        let artists = metadata
            .artists()
            .unwrap_or_default()
            .into_iter()
            .map(str::to_owned)
            .collect();
        let length = metadata
            .length_in_microseconds()
            .map(|us| (us / 1_000_000) as u32)
            .unwrap_or(0);
        let art_url = metadata.art_url().map(str::to_owned);

        SongInfo {
            title,
            album,
            artists,
            length,
            art_url,
        }
    }
}
