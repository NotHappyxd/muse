use mpris::{PlaybackStatus, PlayerFinder};
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedSender};
use tokio::sync::watch::Receiver;
use crate::lyric::{LyricResponse};

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
    PositionChanged {
        progress: u128,
    },
    LyricsFetched {
        lyrics: Option<LyricResponse>,
    },
    Error {
        error: String,
    },
    PlayerCommand(PlayerCommand),
}

#[derive(Debug)]
pub enum PlayerCommand {
    Pause,
    Next,
    Previous
}
const POLL_MS: u64 = 500;

struct WatcherState {
    current_title: Option<String>,
    current_bus: Option<String>,
    current_album: Option<String>,
    album_retry: u32,
}

pub async fn run_watcher(tx: UnboundedSender<AppEvent>, mut shutdown_rx: Receiver<bool>) {
    let mut state = WatcherState {
        current_title: None,
        current_bus: None,
        current_album: None,
        album_retry: 0
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
                p.as_ref()
                    .ok()
                    .and_then(|p| p.get_playback_status().ok())
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
            let _ = tx.send(AppEvent::Idle);
        }

        Some(player) => {
            let bus = player.bus_name().to_owned();

            if state.current_bus.as_deref() != Some(&bus) {
                if state.current_bus.is_some() {
                    let _ = tx.send(AppEvent::PlayerDetached);
                }
                state.current_bus = Some(bus);
                state.current_title = None;
                state.current_album = None;
            }

            if let Ok(pos) = player.get_position() {
                let _ = tx.send(AppEvent::PositionChanged {
                    progress: pos.as_millis(),
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

                    if (different_title && valid_album) || (different_title && !valid_album && state.album_retry >= 3) {
                        state.current_title = Some(title.clone());

                        if !valid_album && state.album_retry >= 3 {
                            state.current_album = Some("".to_string());
                        }else {
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

                        let _ = tx.send(AppEvent::SongChanged { title, album: album.unwrap_or_else(|| "".to_string()), artists, length });
                    }

                    if different_title && !valid_album {
                        state.album_retry += 1
                    }
                }
            }
        }
    }
}