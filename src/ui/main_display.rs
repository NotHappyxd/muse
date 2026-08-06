use crate::ui::state::App;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint::{Fill, Length};
use ratatui::layout::{Alignment, Layout, Rect};
use ratatui::prelude::Text;
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::widgets::{LineGauge, Paragraph, Widget};

impl App {
    pub(crate) fn render_gauge(&self, area: Rect, buf: &mut Buffer) {
        let progress = self.current_progress();
        let label = format_millis(progress);

        let layout = Layout::horizontal([Length(label.len() as u16), Fill(1), Length(8)]);

        let [label_area, gauge_area, end_area] = area.layout(&layout);

        Text::from(label).centered().render(label_area, buf);

        let song_length = match self.active_song.as_ref() {
            Some(song) => song.length,
            None => 0,
        };

        let ratio = if song_length == 0 {
            0.0
        } else {
            progress as f64 / (song_length * 1000) as f64
        };

        let [r, g, b] = self
            .theme
            .main
            .unwrap_or(self.config.color.fallback_main_rgb());

        let filled_style = Color::from([r, g, b]);
        let unfilled_style = Color::from([r / 3, g / 3, b / 3]);

        LineGauge::default()
            .label("")
            .filled_style(Style::default().fg(filled_style))
            .unfilled_style(Style::default().fg(unfilled_style))
            .ratio(ratio.clamp(0.0, 1.0))
            .render(gauge_area, buf);

        Text::raw(format!(" {}", format_second(song_length)))
            .bold()
            .left_aligned()
            .render(end_area, buf);
    }

    pub(crate) fn render_lyrics(&self, area: Rect, buf: &mut Buffer) {
        let Some(active_song) = &self.active_song else {
            return;
        };

        let accent = Color::from(
            self.theme
                .accent
                .unwrap_or(self.config.color.fallback_accent_rgb()),
        );
        let alignment = if self.config.lyric_settings.center {
            Alignment::Center
        } else {
            Alignment::Left
        };

        let Some(lyrics) = &self.lyrics else {
            Text::from("Fetching lyrics...")
                .alignment(alignment)
                .style(Style::default().fg(accent))
                .render(area, buf);
            return;
        };

        if &lyrics.song != &active_song.title {
            Paragraph::new("Fetching lyrics...")
                .alignment(alignment)
                .style(Style::default().fg(accent))
                .render(area, buf);
            return;
        }

        let synced_lyrics = &lyrics.lyrics;

        if synced_lyrics.is_empty() {
            Paragraph::new("No synchronized lyrics found.")
                .alignment(alignment)
                .style(Style::default().fg(accent))
                .render(area, buf);
            return;
        }

        let current_ms = self.current_progress();

        let active_idx = synced_lyrics
            .iter()
            .rposition(|line| line.timestamp <= current_ms as u64)
            .unwrap_or(0);

        let mut lines = Vec::new();

        let mix = self.theme.mix_colors();
        let modifier = if self.config.color.dim_inactive_lines {
            Modifier::DIM
        } else {
            Modifier::BOLD
        };

        for (i, lyric) in synced_lyrics.iter().enumerate() {
            let mut text = lyric.line.clone();

            let style = if i < active_idx {
                Style::default().fg(mix[0]).add_modifier(modifier)
            } else if i == active_idx {
                text.insert_str(0, &self.config.lyric_settings.active_prefix);

                Style::default().fg(accent).bold()
            } else {
                Style::default().fg(mix[1]).add_modifier(modifier)
            };

            lines.push(ratatui::text::Line::from(text).style(style));
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
            .alignment(alignment)
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
