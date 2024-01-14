use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use rq_core::parser::variables::TemplateString;

use crate::components::menu::MenuItem;

impl MenuItem for (String, TemplateString) {
    fn render(&self) -> Vec<ratatui::text::Line<'_>> {
        vec![Line::from(vec![
            Span::raw("@"),
            Span::styled(self.0.as_str(), Style::default().fg(Color::Blue)),
            Span::raw(" = "),
            Span::raw(self.1.to_string()),
        ])]
    }
}
