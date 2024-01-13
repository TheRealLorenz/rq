use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use rq_core::parser::HttpRequest;

use super::menu::MenuItem;

impl MenuItem for HttpRequest {
    fn render(&self) -> Vec<ratatui::text::Line<'_>> {
        let mut lines = Vec::new();

        let mut first_line_spans = vec![
            Span::styled(self.method.to_string(), Style::default().fg(Color::Green)),
            Span::raw(" "),
            Span::raw(self.url.as_str()),
        ];
        let version_span = Span::raw(format!(" {:?}", self.version));

        let mut query = self
            .query
            .iter()
            .enumerate()
            .map(|(i, (k, v))| {
                Line::from(vec![
                    Span::raw(" ".repeat(self.method.to_string().len() + 1)),
                    Span::styled(
                        if i == 0 { "?" } else { "&" },
                        Style::default().fg(Color::Blue),
                    ),
                    Span::raw(k),
                    Span::raw("="),
                    Span::raw(v),
                ])
            })
            .collect::<Vec<_>>();

        if query.is_empty() {
            first_line_spans.push(version_span);
            lines.push(Line::from(first_line_spans));
        } else {
            lines.push(Line::from(first_line_spans));
            query.last_mut().unwrap().spans.push(version_span);
            lines.extend(query);
        }

        let headers: Vec<Line> = self
            .headers()
            .iter()
            .map(|(k, v)| {
                Line::from(vec![
                    Span::styled(k.to_string(), Style::default().fg(Color::Blue)),
                    Span::raw(": "),
                    Span::raw(v.to_str().unwrap().to_string()),
                ])
            })
            .collect();
        lines.extend(headers);

        if !self.body.is_empty() {
            lines.push(Line::styled(
                "Focus to show body",
                Style::default()
                    .fg(Color::Rgb(246, 133, 116))
                    .add_modifier(Modifier::ITALIC),
            ));
        }

        lines.push(Line::from(""));
        lines
    }

    fn render_highlighted(&self) -> Vec<Line<'_>> {
        let mut lines = self.render();

        // Underline first line
        lines[0].patch_style(
            Style::default()
                .add_modifier(Modifier::UNDERLINED)
                .add_modifier(Modifier::BOLD),
        );

        // Replace body with expanded version
        if !self.body.is_empty() {
            lines.pop();
            lines.pop();

            for line in self.body.lines() {
                lines.push(Line::styled(
                    line,
                    Style::default().fg(Color::Rgb(246, 133, 116)),
                ));
            }
            lines.push(Line::from(""));
        }

        lines
    }
}
