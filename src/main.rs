mod cache;
mod colorgen;
mod config;
mod lyric;
mod theme;
mod ui;
mod watcher;

use crate::config::Config;
use crate::ui::state::{App, Song};
use crate::watcher::PlayerCommand;
use color_eyre::Result;
use mpris::PlayerFinder;
use ratatui::DefaultTerminal;
use std::cell::Cell;
use std::io::stdout;
use std::time::Duration;
use crossterm::event::EnableMouseCapture;
use crossterm::execute;
use crossterm::terminal::EnterAlternateScreen;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::watch;
use watcher::{run_watcher, AppEvent};

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::init();
    color_eyre::install()?;

    let (tx, mut rx) = unbounded_channel();
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let watcher_tx = tx.clone();
    let config_clone = config.clone();
    tokio::spawn(async move {
        run_watcher(watcher_tx, shutdown_rx, &config_clone).await;
    });

    let result = tokio::task::spawn_blocking(move || {
        let _ = execute!(
            stdout(),
            EnterAlternateScreen,
            EnableMouseCapture
        );

        let _ = ratatui::run(|terminal| run_ui(terminal, &mut rx, tx, config));
    })
    .await?;

    let _ = shutdown_tx.send(true);

    Ok(result)
}

fn run_ui(
    terminal: &mut DefaultTerminal,
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<AppEvent>,
    tx: UnboundedSender<AppEvent>,
    config: Config,
) -> Result<()> {
    let mut app = App::new(config);

    app.tx = Some(tx.clone());

    loop {
        while let Ok(event) = rx.try_recv() {
            handle_event(event, &mut app)
        }

        terminal.draw(|frame| match app.handle_events() {
            Ok(false) => {
                frame.render_widget(&mut app, frame.area());
            }
            Ok(true) => {}
            Err(e) => {
                eprintln!("Error {}", e);
            }
        })?;

        if app.quit {
            break;
        }
    }

    Ok(())
}

fn handle_event(event: AppEvent, app: &mut App) {
    match event {
        AppEvent::SongChanged {
            title,
            album,
            artists,
            length,
        } => {
            let song = Song::new(title, album, artists, length);

            app.active_song = Some(song);

            if app.config.color.generate {
                let song = app.active_song.as_ref().unwrap();

                if app.theme.title.as_ref() != Some(&song.title) {
                    app.theme.reset();
                }
            }

            app.lyrics = None;
            app.manual_scroll_offset = Cell::new(0);
        }

        AppEvent::PlaybackAnchor {
            position_ms,
            is_playing,
            at,
        } => app.set_progress(position_ms, is_playing, at),

        AppEvent::PlayerDetached => {}

        AppEvent::Error { error: _error } => {
            panic!("{}", _error)
        }

        AppEvent::LyricsFetched { lyrics } => {
            app.lyrics = lyrics;
        }

        AppEvent::Idle => {}
        AppEvent::PlayerCommand(command) => handle_player_command(command),
        AppEvent::ThemeFetched { song_title, theme } => {
            app.theme.title = Some(song_title);
            app.theme.main = Some(theme.main);
            app.theme.accent = Some(theme.accent);

            if app.config.color.theme_inactive_lines {
                app.theme.inactive = theme.inactive;
                app.theme.upcoming = theme.upcoming;
            }
        }
    }
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
            PlayerCommand::Pause => {
                let _ = player.play_pause();
            }
            PlayerCommand::Next => {
                let _ = player.next();
            }
            PlayerCommand::Previous => {
                let _ = player.previous();
            }
            PlayerCommand::Skip(timestamp) => {
                if let Some(track_id) = player.get_metadata().ok().and_then(|m| m.track_id()) {
                    let _ = player.set_position(track_id, &Duration::from_millis(timestamp));
                }
            }
        }
    }
}
