use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::BlockComponent;

#[derive(Clone)]
pub struct Legend {
    keymaps: Vec<&'static (&'static str, &'static str)>,
}

impl Legend {
    pub fn new<I: Iterator<Item = &'static (&'static str, &'static str)>>(keymaps: I) -> Self {
        Self {
            keymaps: keymaps.collect(),
        }
    }
}

impl BlockComponent for Legend {
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
