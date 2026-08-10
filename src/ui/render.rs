use crate::ui::header::header;
use crate::ui::state::App;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint::{Length, Min};
use ratatui::layout::{Layout, Margin, Rect};
use ratatui::widgets::Widget;

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::vertical([Length(1), Min(0)]);

        let [header_area, main_area] = area.layout(&layout);

        let main_layout = Layout::vertical([
            Length(2), // Progress gauge
            Min(0),    // Lyrics area
        ]);

        let [gauge_area, lyrics_area] = main_area.inner(Margin::new(1, 0)).layout(&main_layout);

        header(header_area, buf, self);
        self.render_gauge(gauge_area, buf);
        
        self.lyric_area = Some(lyrics_area);
        
        self.render_lyrics(lyrics_area, buf);
    }
}
