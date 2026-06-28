use std::time::Duration;
use crossterm::event;
use crossterm::event::{KeyCode, KeyModifiers};
use crate::ui::state::App;

impl App {
    pub(crate) fn handle_events(&mut self) -> color_eyre::Result<bool> {
        let timeout = Duration::from_secs_f32(1.0 / 20.0);
        if event::poll(timeout)?
            && let Some(key) = event::read()?.as_key_press_event()
        {
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                self.quit = true;
                return Ok(true)
            }

            match key.code {
                KeyCode::Char('q') => return {
                    self.quit = true;
                    Ok(true)
                },
                KeyCode::Up | KeyCode::Char('k') => {
                    self.manual_scroll_offset.set(self.manual_scroll_offset.get().saturating_sub(1));
                },
                KeyCode::Down | KeyCode::Char('j') => {
                    self.manual_scroll_offset.set(self.manual_scroll_offset.get().saturating_add(1));
                },
                _ => {}
            }
        }
        Ok(false)
    }
}