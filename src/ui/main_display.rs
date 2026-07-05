use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Layout, Rect};
use ratatui::layout::Constraint::{Fill, Length};
use ratatui::prelude::Text;
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{LineGauge, Paragraph, Widget};
use crate::config::Config;
use crate::lyric::LyricLine;
use crate::ui::state::App;

impl App {
    pub(crate) fn render_gauge(&self, area: Rect, buf: &mut Buffer) {
        let progress = self.current_progress();
        let label = format_millis(progress);

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

        let ratio = if song_length == 0 { 0.0 } else { progress as f64 / (song_length * 1000) as f64 };

        let [r, g, b] = self.song_theme;

        let filled_style = if r == 0 && b == 0 && g == 0 { Color::Indexed(149) } else { Color::from(self.song_theme)};
        let unfilled_style = if r == 0 && b == 0 && g == 0 { Color::Indexed(58) } else { Color::from([r / 3, g / 3, b / 3])};

        LineGauge::default()
            .label("")
            .filled_style(
                Style::default()
                    .fg(filled_style)
            )
            .unfilled_style(
                Style::default()
                    .fg(unfilled_style)
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

        let accent = Color::from(self.song_accent);

        let Some(lyrics) = &self.lyrics else {
            Text::from("Fetching lyrics...")
                .centered()
                .style(Style::default().fg(accent))
                .render(area, buf);
            return;
        };

        if &lyrics.song != &active_song.title {
            Paragraph::new("Fetching lyrics...")
                .centered()
                .style(Style::default().fg(accent))
                .render(area, buf);
            return;
        }

        let synced_lyrics = &lyrics.lyrics;

        if synced_lyrics.is_empty() {
            Paragraph::new("No synchronized lyrics found.")
                .centered()
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

        for (i, lyric) in synced_lyrics.iter().enumerate() {
            if i == active_idx {
                lines.push(render_active_line(lyric, current_ms, accent, &self.config));
            } else {
                let style = if i < active_idx {
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM)
                } else {
                    Style::default().fg(Color::Gray).add_modifier(Modifier::DIM)
                };

                lines.push(Line::from(lyric.line.clone()).style(style));
            }
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

fn render_active_line<'a>(lyric: &'a LyricLine, current_ms: u128, accent: Color, config: &'a Config) -> Line<'a> {
    if lyric.words.is_empty() {
        let text = [config.active_lyric_prefix.as_str(), lyric.line.as_str()].concat();
        return Line::from(text)
            .style(Style::default().fg(accent).bold());
    }

    let sung_style = Style::default().fg(accent).add_modifier(Modifier::DIM);
    let current_style = Style::default().fg(accent).bold();
    let upcoming_style = Style::default().fg(Color::Gray).add_modifier(Modifier::DIM);

    let mut spans: Vec<Span> = Vec::with_capacity(lyric.words.len() * 2 + 1);

    let active_idx = lyric.words.iter()
        .rposition(|word| word.start as u128 <= current_ms)
        .unwrap_or(0);

    for (i, word) in lyric.words.iter().enumerate() {
        let style = if i < active_idx {
            sung_style
        } else if i == active_idx {
            if word.end as u128 + 30 <= current_ms {
                sung_style
            }else {
                current_style
            }
        } else {
            upcoming_style
        };

        if i > 0 {
            spans.push(Span::raw(" "));
        }

        spans.push(Span::styled(word.text.as_str(), style));
    }

    spans.insert(0, Span::styled(config.active_lyric_prefix.as_str(), current_style));
    Line::from(spans)
}


fn format_millis(millis: u128) -> String {
    let second = millis / 1000;
    format_second(second as u32)
}

fn format_second(second: u32) -> String {
    let minute = second / 60;

    format!("{}:{:02}", minute, second % 60)
}