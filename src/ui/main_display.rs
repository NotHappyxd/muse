use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Layout, Rect};
use ratatui::layout::Constraint::{Fill, Length};
use ratatui::prelude::Text;
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::widgets::{LineGauge, Paragraph, Widget};
use crate::ui::state::App;

impl App {
    pub(crate) fn render_gauge(&self, area: Rect, buf: &mut Buffer) {
        let label = format_millis(self.progress);

        let layout = Layout::horizontal([
            Length(label.len() as u16),
            Fill(1),
            Length(8),
        ]);

        let [label_area, gauge_area, end_area] =
            area.layout(&layout);

        Text::from(label)
            .centered()
            .render(label_area, buf);

        let song_length = match self.active_song.as_ref() {
            Some(song) => song.length,
            None => 0
        };

        let ratio = if song_length == 0 { 0.0 } else { self.progress as f64 / (song_length * 1000) as f64 };

        LineGauge::default()
            .label("")
            .filled_style(
                Style::default()
                    .fg(Color::Indexed(149))
            )
            .unfilled_style(
                Style::default()
                    .fg(Color::Indexed(58))
            )
            .ratio(ratio.clamp(0.0, 1.0))
            .render(gauge_area, buf);


        Text::raw(format!(" {}", format_second(song_length)))
            .bold()
            .left_aligned()
            .render(end_area, buf);
    }

    pub(crate) fn render_lyrics(&self, area: Rect, buf: &mut Buffer) {
        let Some(active_song) = &self.active_song else {
            return
        };

        let Some(lyrics) = &self.lyrics else {
            Text::from("Fetching lyrics...")
                .centered()
                .render(area, buf);
            return;
        };

        if &lyrics.song != &active_song.title {
            Paragraph::new("Fetching lyrics...")
                .centered()
                .render(area, buf);
            return;
        }

        let synced_lyrics = &lyrics.lyrics;

        if synced_lyrics.is_empty() {
            Paragraph::new("No synchronized lyrics found.")
                .centered()
                .render(area, buf);
            return;
        }

        let current_ms = self.progress;

        let active_idx = synced_lyrics
            .iter()
            .rposition(|line| line.timestamp <= current_ms as u64)
            .unwrap_or(0);

        let mut lines = Vec::new();

        for (i, lyric) in synced_lyrics.iter().enumerate() {
            let style = if i < active_idx {
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM)
            } else if i == active_idx {
                Style::default().fg(Color::White).bold()
            } else {
                Style::default().fg(Color::Gray).add_modifier(Modifier::DIM)
            };

            lines.push(ratatui::text::Line::from(lyric.line.clone()).style(style));
        }

        let visible_height = area.height;
        let half_height = visible_height.saturating_sub(1) / 2;

        let base_scroll_y = active_idx.saturating_sub(half_height as usize) as i16;
        let max_scroll = lines.len().saturating_sub(visible_height as usize).max(0) as i16;

        let mut offset = self.manual_scroll_offset.get();
        let mut final_scroll = base_scroll_y + self.manual_scroll_offset.get();

        if final_scroll < 0 {
            offset += 0 - final_scroll;
            final_scroll = 0;
        }

        if final_scroll > max_scroll {
            offset -= final_scroll - max_scroll;
            final_scroll = max_scroll;
        }

        self.manual_scroll_offset.set(offset);

        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .scroll((final_scroll as u16, 0))
            .render(area, buf);
    }
}

fn format_millis(millis: u128) -> String {
    let second = millis / 1000;
    format_second(second as u32)
}

fn format_second(second: u32) -> String {
    let minute = second / 60;

    format!("{}:{:02}", minute, second % 60)
}