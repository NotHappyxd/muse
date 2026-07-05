use crate::lyric::{LyricResponse, fetch_lyric};
use crate::theme::fetch_theme;
use mpris::{PlaybackStatus, PlayerFinder};
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
        rgb: [u8; 3],
        accent: [u8; 3],
    },
}

#[derive(Debug)]
pub enum PlayerCommand {
    Pause,
    Next,
    Previous,
}
const POLL_MS: u64 = 500;
const MAXIMUM_DRIFT_ALLOWED: i128 = 750;

struct WatcherState {
    current_title: Option<String>,
    current_bus: Option<String>,
    current_album: Option<String>,
    album_retry: u32,
    anchor_position: Option<u128>,
    anchor_instant: Option<Instant>,
    anchor_playing: bool,
}

pub async fn run_watcher(tx: UnboundedSender<AppEvent>, mut shutdown_rx: Receiver<bool>) {
    let mut state = WatcherState {
        current_title: None,
        current_bus: None,
        current_album: None,
        album_retry: 0,
        anchor_position: None,
        anchor_instant: None,
        anchor_playing: false,
    };

    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    break;
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(POLL_MS)) => {
                poll(&tx, &mut state);
            }
        }
    }
}

fn poll(tx: &UnboundedSender<AppEvent>, state: &mut WatcherState) {
    let finder = match PlayerFinder::new() {
        Ok(f) => f,
        Err(e) => {
            let _ = tx.send(AppEvent::Error {
                error: format!("D-Bus connection failed: {e}"),
            });
            return;
        }
    };

    let active = finder
        .iter_players()
        .ok()
        .and_then(|mut it| {
            it.find(|p| {
                p.as_ref().ok().and_then(|p| p.get_playback_status().ok())
                    == Some(PlaybackStatus::Playing)
            })
        })
        .and_then(|p| p.ok());

    match active {
        None => {
            if state.current_bus.is_some() {
                state.current_bus = None;
                state.current_title = None;
                state.current_album = None;
                let _ = tx.send(AppEvent::PlayerDetached);
            }

            if state.anchor_instant.is_some() || state.anchor_playing {
                let estimated_position = predicted_position(state, Instant::now());
                state.anchor_position = Some(estimated_position);
                state.anchor_instant = None;
                state.anchor_playing = false;

                let _ = tx.send(AppEvent::PlaybackAnchor {
                    position_ms: estimated_position,
                    at: Instant::now(),
                    is_playing: false,
                });
            }

            let _ = tx.send(AppEvent::Idle);
        }

        Some(player) => {
            let bus = player.bus_name().to_owned();
            let new_session = state.anchor_position.is_none() && !state.anchor_playing;

            if state.current_bus.as_deref() != Some(&bus) {
                if state.current_bus.is_some() {
                    let _ = tx.send(AppEvent::PlayerDetached);
                }
                state.current_bus = Some(bus);
                state.current_title = None;
                state.current_album = None;
            }

            let now = Instant::now();
            let (poll_ms, have_real_reading) = match player.get_position() {
                Ok(pos) => (pos.as_millis(), true),
                Err(_) => (predicted_position(state, now), false),
            };

            let drifted = have_real_reading && !new_session && {
                let predicted = predicted_position(state, now);
                (poll_ms as i128 - predicted as i128).abs() > MAXIMUM_DRIFT_ALLOWED
            };

            if new_session || drifted {
                state.anchor_position = Some(poll_ms);
                state.anchor_instant = Some(now);
                state.anchor_playing = true;

                let _ = tx.send(AppEvent::PlaybackAnchor {
                    position_ms: poll_ms,
                    is_playing: true,
                    at: now,
                });
            }

            match player.get_metadata() {
                Err(e) => {
                    let _ = tx.send(AppEvent::Error {
                        error: format!("Failed to get metadata: {e}"),
                    });
                }

                Ok(metadata) => {
                    let title = metadata.title().unwrap_or("Unknown").to_owned();

                    let raw_album = metadata.album_name();
                    let album = raw_album
                        .filter(|a| !a.trim().is_empty() && *a != "Unknown")
                        .map(|a| a.to_owned());

                    let different_title = state.current_title.as_ref() != Some(&title);
                    let valid_album = album.is_some();

                    if (different_title && valid_album)
                        || (different_title && !valid_album && state.album_retry >= 3)
                    {
                        state.current_title = Some(title.clone());

                        if !valid_album && state.album_retry >= 3 {
                            state.current_album = Some("".to_string());
                        } else {
                            state.current_album = album.clone();
                        }

                        state.album_retry = 0;

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

                        if let Some(art_url) = metadata.art_url().map(str::to_owned) {
                            let album_str = album.clone().unwrap_or_else(|| "".to_string());
                            let artists_clone = Vec::clone(&artists);
                            let title_clone = title.clone();
                            let tx2 = tx.clone();
                            tokio::spawn(async move {
                                let lyric_future =
                                    fetch_lyric(&title_clone, &artists_clone, &album_str, length);
                                let theme_future = fetch_theme(art_url, &tx2);

                                let (lyrics, _) = tokio::join!(lyric_future, theme_future);
                                let _ = tx2.send(AppEvent::LyricsFetched { lyrics });
                            });
                        }

                        let _ = tx.send(AppEvent::SongChanged {
                            title,
                            album: album.unwrap_or_else(|| "".to_string()),
                            artists,
                            length,
                        });
                    }

                    if different_title && !valid_album {
                        state.album_retry += 1
                    }
                }
            }
        }
    }
}

fn predicted_position(state: &WatcherState, now: Instant) -> u128 {
    let Some(anchor_position) = state.anchor_position else {
        return 0;
    };

    if state.anchor_playing {
        if let Some(anchor_at) = state.anchor_instant {
            return anchor_position + now.saturating_duration_since(anchor_at).as_millis();
        }
    }

    anchor_position
}
