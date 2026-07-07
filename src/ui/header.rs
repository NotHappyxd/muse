use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::widgets::{Paragraph, Widget};
use crate::ui::state::App;

pub fn header(area: Rect, buf: &mut Buffer, app: &App) {
    let text = match app.active_song.as_ref() {
        Some(song) => {
            let raw_header = app.config.header.title.replace("{title}", &song.title)
                .replace("{artists}", &song.artists[0])
                .replace("{album}", &song.album);

            raw_header
        },
        None => String::from("Cannot detect song!")
    };

    let mut header = Paragraph::new(text)
        .bold()
        .left_aligned();

    if app.config.header.centered {
        header = header.centered()
    }

    header.render(area, buf);
}
