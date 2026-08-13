use crate::lyric::LyricLine;
use crate::ui::state::App;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint::{Fill, Length};
use ratatui::layout::{Alignment, Layout, Rect};
use ratatui::prelude::Text;
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{LineGauge, Paragraph, Widget};
use std::cmp::Ordering;

impl App {
    pub(crate) fn render_gauge(&self, area: Rect, buf: &mut Buffer) {
        let progress = self.current_progress();
        let label = format_millis(progress);

        let layout = Layout::horizontal([Length(label.len() as u16), Fill(1), Length(8)]);

        let [label_area, gauge_area, end_area] = area.layout(&layout);

        Text::from(label).centered().render(label_area, buf);

        let song_length = self.active_song.as_ref().map_or(0, |song| song.length);

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
        let accent = self
            .theme
            .accent
            .map(Color::from)
            .unwrap_or(Color::from(self.config.color.fallback_accent_rgb()));
        
        if let Some(debug_msg) = &self.debug_text {
            Text::from(debug_msg.clone())
                .centered()
                .style(Style::default().fg(accent))
                .render(area, buf);
            return;
        }
        
        let Some(active_song) = &self.active_song else {
            return;
        };
        
        let alignment = if self.config.lyric.center {
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
            Text::from("Fetching lyrics...")
                .alignment(alignment)
                .style(Style::default().fg(accent))
                .render(area, buf);
            return;
        }

        let synced_lyrics = &lyrics.lyrics;

        if synced_lyrics.is_empty() {
            Text::from("No synchronized lyrics found.")
                .alignment(alignment)
                .style(Style::default().fg(accent))
                .render(area, buf);
            return;
        }

        self.display_lyrics(area, buf, synced_lyrics, accent, alignment)
    }

    fn display_lyrics(
        &self,
        area: Rect,
        buf: &mut Buffer,
        synced_lyrics: &[LyricLine],
        accent: Color,
        alignment: Alignment,
    ) {
        let current_ms = self.current_progress();

        let active_idx = synced_lyrics
            .iter()
            .rposition(|line| line.timestamp <= current_ms as u64);

        let mut lines = Vec::with_capacity(synced_lyrics.len());

        let (inactive, upcoming) = self.theme.mix_colors();

        let modifier = if self.config.color.dim_inactive_lines {
            Modifier::DIM
        } else {
            Modifier::BOLD
        };

        for (i, lyric) in synced_lyrics.iter().enumerate() {
            let mut text = lyric.line.clone();

            let style = match active_idx {
                Some(active_idx) => match i.cmp(&active_idx) {
                    Ordering::Less => Style::default().fg(inactive).add_modifier(modifier),
                    Ordering::Equal => Style::default().fg(accent).bold(),
                    Ordering::Greater => Style::default().fg(upcoming).add_modifier(modifier),
                },
                None => Style::default().fg(upcoming).add_modifier(modifier),
            };

            if let Some(active_idx) = active_idx
                && active_idx == i
            {
                text.insert_str(0, &self.config.lyric.active_prefix);
            }

            lines.push(Line::from(text).style(style));
        }

        let visible_height = area.height;

        let final_scroll = self.lyric_final_scroll_pos(visible_height, lines.len());

        Paragraph::new(lines)
            .alignment(alignment)
            .scroll((final_scroll as u16, 0))
            .render(area, buf);
    }

    fn lyric_scroll_position(&self, visible_height: u16) -> i16 {
        let Some(lyrics) = &self.lyrics else {
            return 0;
        };

        let current_ms = self.current_progress();

        let active_idx = lyrics
            .lyrics
            .iter()
            .rposition(|line| line.timestamp <= current_ms as u64)
            .unwrap_or(0);

        let half_height = visible_height.saturating_sub(1) / 2;

        active_idx.saturating_sub(half_height as usize) as i16
    }

    fn lyric_final_scroll_pos(&self, visible_height: u16, line_count: usize) -> i16 {
        let max_scroll = line_count.saturating_sub(visible_height as usize) as i16;
        let mut offset = self.manual_scroll_offset.get();
        let mut final_scroll = self.lyric_scroll_position(visible_height) + offset;

        if final_scroll < 0 {
            offset -= final_scroll;
            final_scroll = 0;
        }

        if final_scroll > max_scroll {
            offset -= final_scroll - max_scroll;
            final_scroll = max_scroll;
        }

        self.manual_scroll_offset.set(offset);

        final_scroll
    }

    pub fn lyric_at(&self, x: u16, y: u16) -> Option<usize> {
        let area = self.lyric_area?;

        if !area.contains((x, y).into()) {
            return None;
        }

        let lyrics = self.lyrics.as_ref()?;

        if lyrics.lyrics.is_empty() {
            return None;
        }

        let final_scroll = self.lyric_final_scroll_pos(area.height, lyrics.lyrics.len());
        let relative_y = y.saturating_sub(area.y);
        let lyric_index = final_scroll as usize + relative_y as usize;

        (lyric_index < lyrics.lyrics.len()).then_some(lyric_index)
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
