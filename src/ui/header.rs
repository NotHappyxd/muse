use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::widgets::{Paragraph, Widget};
use crate::ui::state::App;

pub fn header(area: Rect, buf: &mut Buffer, app: &App) {
    let text = match app.active_song.as_ref() {
        Some(song) => format!(" {} - {} ({})", song.title, song.artists[0], song.album),
        None => "Cannot detect song!".to_string()
    };

    let header = Paragraph::new(text)
        .bold()
        .left_aligned();

    header.render(area, buf);
}
