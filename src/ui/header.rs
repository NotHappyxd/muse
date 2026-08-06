use crate::ui::state::App;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::widgets::{Paragraph, Widget};

pub fn header(area: Rect, buf: &mut Buffer, app: &App) {
    let text = match app.active_song.as_ref() {
        Some(song) => {
            let artists = if song.artists.is_empty() {
                ""
            } else {
                &song.artists[0]
            };

            let raw_header = app
                .config
                .header
                .title
                .replace("{title}", &song.title)
                .replace("{artists}", artists)
                .replace("{album}", &song.album);

            raw_header
        }
        None => String::from("Cannot detect song!"),
    };

    let mut header = Paragraph::new(text).bold().left_aligned();

    if app.config.header.centered {
        header = header.centered()
    }

    header.render(area, buf);
}
