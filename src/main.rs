mod ui;
mod watcher;
mod lyric;
mod cache;

use std::cell::Cell;
use ratatui::DefaultTerminal;
use color_eyre::Result;
use mpris::{PlayerFinder};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::watch;
use watcher::{run_watcher, AppEvent};
use crate::lyric::fetch_lyric;
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

    let result = ratatui::run(|terminal| {
        run_ui(terminal, &mut rx, tx)
    });

    let _ = shutdown_tx.send(true);

    result
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

                    let song_clone = song.clone();
                    let tx_clone = tx.clone();

                    tokio::spawn(async move {
                        let response = fetch_lyric(&song_clone).await;

                        let _ = tx_clone.send(AppEvent::LyricsFetched { lyrics: response });
                    });

                    app.active_song = Some(song);
                    app.lyrics = None;
                    app.manual_scroll_offset = Cell::new(0)
                }

                AppEvent::PositionChanged {
                    progress,
                } => {
                    app.set_progress(progress);
                }

                AppEvent::PlayerDetached => {}

                AppEvent::Error { error } => {

                }

                AppEvent::LyricsFetched { lyrics } => {
                    app.lyrics = lyrics;
                }

                AppEvent::Idle => {},
                AppEvent::PlayerCommand(command) => {
                    handle_player_command(command)
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