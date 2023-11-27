use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::BlockComponent;

pub struct Legend<'a> {
    keymaps: Vec<&'a (&'a str, &'a str)>,
}

impl<'a> Legend<'a> {
    pub fn new<I: Iterator<Item = &'a (&'a str, &'a str)>>(keymaps: I) -> Self {
        Self {
            keymaps: keymaps.collect(),
        }
    }
}

impl BlockComponent for Legend<'_> {
    fn render(
        &self,
        frame: &mut crate::terminal::Frame,
        area: ratatui::prelude::Rect,
        block: ratatui::widgets::Block,
    ) {
        let spans = self
            .keymaps
            .iter()
            .flat_map(|(k, v)| {
                [
                    Span::styled(
                        format!(" {k} "),
                        Style::default().add_modifier(Modifier::REVERSED),
                    ),
                    Span::from(format!(" {v} ")),
                ]
            })
            .collect::<Vec<_>>();

        let p = Paragraph::new(Line::from(spans));

        frame.render_widget(p.block(block), area);
    }
}
