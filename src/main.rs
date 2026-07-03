mod ui;
mod watcher;
mod lyric;
mod cache;
mod theme;
mod colorgen;

use std::cell::Cell;
use ratatui::DefaultTerminal;
use color_eyre::Result;
use mpris::{PlayerFinder};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::watch;
use watcher::{run_watcher, AppEvent};
use crate::ui::state::{App, Song};
use crate::watcher::PlayerCommand;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let (tx, mut rx) = unbounded_channel();
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let watcher_tx = tx.clone();
    tokio::spawn(async move {
        run_watcher(watcher_tx, shutdown_rx).await;
    });

    let result = tokio::task::spawn_blocking(move || {
        let _ = ratatui::run(|terminal| {
            run_ui(terminal, &mut rx, tx)
        });
    }).await?;

    let _ = shutdown_tx.send(true);

    Ok(result)
}


fn run_ui(
    terminal: &mut DefaultTerminal,
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<AppEvent>,
    tx: UnboundedSender<AppEvent>,
) -> Result<()> {

    let mut app = App::new();

    app.tx = Some(tx.clone());

    loop {
        while let Ok(event) = rx.try_recv() {
            match event {
                AppEvent::SongChanged {
                    title,
                    album,
                    artists,
                    length,
                } => {
                    let song = Song::new(title, album, artists, length);

                    app.active_song = Some(song);
                    app.song_theme = [0, 0, 0];
                    app.song_accent = [0, 0, 0];
                    app.lyrics = None;
                    app.manual_scroll_offset = Cell::new(0)
                }

                AppEvent::PlaybackAnchor { position_ms, is_playing, at } => {
                    app.set_progress(position_ms, is_playing, at)
                }

                AppEvent::PlayerDetached => {}

                AppEvent::Error { error: _error } => {

                }

                AppEvent::LyricsFetched { lyrics } => {
                    app.lyrics = lyrics;
                }

                AppEvent::Idle => {},
                AppEvent::PlayerCommand(command) => {
                    handle_player_command(command)
                },
                AppEvent::ThemeFetched { rgb, accent } => {
                    app.song_theme = rgb;
                    app.song_accent = accent;
                }
            }
        }

        terminal.draw(|frame| {
            match app.handle_events() {
                Ok(false) => {
                    frame.render_widget(
                        &app,
                        frame.area()
                    );
                }
                Ok(true) => {}
                Err(e) => {
                    eprintln!("Error {}", e);
                }
            }
        })?;

        if app.quit {
            break;
        }
    }

    Ok(())
}

fn handle_player_command(cmd: PlayerCommand) {
    let finder = match PlayerFinder::new() {
        Ok(f) => f,
        Err(_) => return,
    };

    let player = finder
        .iter_players()
        .ok()
        .and_then(|mut it| it.find_map(|p| p.ok()));

    if let Some(player) = player {
        match cmd {
            PlayerCommand::Pause => { let _ = player.play_pause(); }
            PlayerCommand::Next => { let _ = player.next(); }
            PlayerCommand::Previous => {
                let _ = player.previous();
            }
        }
    }
}