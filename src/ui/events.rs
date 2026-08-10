use crate::ui::state::App;
use crate::watcher::{AppEvent, PlayerCommand};
use crossterm::event;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use std::time::Duration;

impl App {
    pub(crate) fn handle_events(&mut self) -> color_eyre::Result<bool> {
        let timeout = Duration::from_secs_f32(1.0 / 20.0);
        
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    let res = self.handle_key_press(key);

                    if let Ok(true) = res {
                        return Ok(true);
                    }
                },
                Event::Mouse(mouse) => self.handle_mouse_press(mouse),
                _ => {

                }
            }
        }

        Ok(false)
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> color_eyre::Result<bool> {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.quit = true;
            return Ok(true);
        }

        match key.code {
            KeyCode::Char('q') => {
                return {
                    self.quit = true;
                    Ok(true)
                };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.manual_scroll_offset
                    .set(self.manual_scroll_offset.get().saturating_sub(1));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.manual_scroll_offset
                    .set(self.manual_scroll_offset.get().saturating_add(1));
            }
            KeyCode::Char(' ') => {
                if let Some(tx) = &self.tx {
                    let _ = tx.send(AppEvent::PlayerCommand(PlayerCommand::Pause));
                }
            }

            KeyCode::Right => {
                if let Some(tx) = &self.tx {
                    let _ = tx.send(AppEvent::PlayerCommand(PlayerCommand::Next));
                }
            }

            KeyCode::Left => {
                if let Some(tx) = &self.tx {
                    let _ = tx.send(AppEvent::PlayerCommand(PlayerCommand::Previous));
                }
            }
            _ => {}
        }

        Ok(false)
    }

    fn handle_mouse_press(&mut self, mouse: MouseEvent) {
        if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
            return;
        }

        let x = mouse.column;
        let y = mouse.row;

        let Some(at_idx) = self.lyric_at(x, y) else {
            return;
        };
        
        let Some(lyric) = self.lyrics.as_ref() else {
            return;
        };

        let timestamp = lyric.lyrics.get(at_idx).unwrap().timestamp;
        if let Some(tx) = &self.tx {
            let _ = tx.send(AppEvent::PlayerCommand(PlayerCommand::Skip(timestamp)));
        }
    }
}
