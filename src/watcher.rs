use std::fmt::format;
use crate::colorgen::generator::Theme;
use crate::config;
use crate::config::Config;
use crate::lyric::{LyricResponse, fetch_lyric};
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
    Skip(u64),
}
const POLL_MS: u64 = 500;
const RETRY_COUNT: u8 = 3;

#[derive(Default)]
struct PlaybackAnchor {
    position: Option<u128>,
    instant: Option<Instant>,
    playing: bool,
    last_real_position: Option<u128>,
    last_change_instant: Option<Instant>,
    estimated_tick_rate: u128
}

#[derive(Default)]
struct WatcherState {
    current_title: Option<String>,
    current_bus: Option<String>,
    current_album: Option<String>,
    album_retry: u8,
    playback_anchor: PlaybackAnchor
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

    let anchor = &state.playback_anchor;

    if anchor.instant.is_some() || anchor.playing {
        let now = Instant::now();
        let estimated_position = anchor.predicted_position(now);
        state.playback_anchor.pause(estimated_position);

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


    let paused_before_sync = !state.playback_anchor.playing;
    sync_playback_drift(player, state, tx);

    let anchor = &state.playback_anchor;
    let was_paused = anchor.playing && paused_before_sync;

    match player.get_metadata() {
        Err(e) => {
            let _ = tx.send(AppEvent::Error {
                error: format!("Failed to get metadata: {e}"),
            });
        }

        Ok(metadata) => handle_metadata(&metadata, state, config, tx, was_paused),
    }
}

const MAXIMUM_DRIFT_ALLOWED: u64 = 750;
const DEFAULT_TICK_ESTIMATED_MS: u128 = POLL_MS as u128;

fn sync_playback_drift(player: &Player, state: &mut WatcherState, tx: &UnboundedSender<AppEvent>) {
    let now = Instant::now();
    let anchor = &mut state.playback_anchor;
    let Ok(reported_position) = player.get_position() else { return; };
    let reported_ms = reported_position.as_millis();

    let paused = matches!(player.get_playback_status(), Ok(PlaybackStatus::Paused));

    if paused {
        anchor.pause(reported_ms);
        anchor.report(now, &tx);
        return;
    }

    let is_new_reading = anchor.last_real_position != Some(reported_ms);
    if !is_new_reading {
        return;
    }

    let is_new_session = !anchor.playing;
    if is_new_session { // Trust the result of a new session implicitly
        anchor.update(reported_ms, now, true, reported_ms);
        anchor.report(now, &tx);
        return;
    }

    let predicted = anchor.predicted_position(now);
    let delta = predicted as i128 - reported_ms as i128;
    let is_seek = delta.abs() > MAXIMUM_DRIFT_ALLOWED as i128;

    if is_seek {
        anchor.update(reported_ms, now, true, reported_ms);
        anchor.report(now, &tx);
        return;
    }

    let gap = now.saturating_duration_since(anchor.last_change_instant.unwrap_or(now)).as_millis();

    const TICK_RATE_WEIGHT: u128 = 5;
    const GAP_WEIGHT: u128 = 5;
    let clamped_gap = gap.clamp(POLL_MS as u128, 5_000);

    anchor.estimated_tick_rate = (anchor.estimated_tick_rate * TICK_RATE_WEIGHT + clamped_gap * GAP_WEIGHT) / 10;
    let compensation = (reported_ms + anchor.estimated_tick_rate / 2).max(predicted);

    anchor.update(compensation, now, true, reported_ms);
    anchor.report(now, &tx);
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
            min_chroma_percentage,
        )
        .await
    });
}

impl WatcherState {
    fn reset_session(&mut self) {
        self.current_bus = None;
        self.current_title = None;
        self.current_album = None;
        self.playback_anchor.estimated_tick_rate = DEFAULT_TICK_ESTIMATED_MS;
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

impl PlaybackAnchor {
    fn update(&mut self, position: u128, now: Instant, is_playing: bool, last_real_position: u128) {
        self.position = Some(position);
        self.instant = Some(now);
        self.playing = is_playing;
        self.last_change_instant = Some(now);
        self.last_real_position = Some(last_real_position);
    }

    fn pause(&mut self, position: u128) {
        self.position = Some(position);
        self.instant = None;
        self.playing = false;
        self.last_real_position = None;
        self.last_change_instant = None;
    }

    fn report(&self, now: Instant, tx: &UnboundedSender<AppEvent>) {
        if let Some(position) = self.position {
            let _ = tx.send(AppEvent::PlaybackAnchor { position_ms: position, is_playing: self.playing, at: now });
        }
    }

    fn predicted_position(&self, now: Instant) -> u128 {
        let Some(anchor_position) = self.position else {
            return 0;
        };

        if let (true, Some(anchor_at)) = (self.playing, self.instant) {
            return anchor_position + now.saturating_duration_since(anchor_at).as_millis();
        }

        anchor_position
    }
}