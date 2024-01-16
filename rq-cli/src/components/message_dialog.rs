use ratatui::{
    style::{Color, Style},
    widgets::{Paragraph, Wrap},
};

use crate::event::Message;

use super::{BlockComponent, HandleResult, HandleSuccess};

pub struct MessageDialog {
    content: Message,
}
impl MessageDialog {
    pub fn new(message: Message) -> Self {
        Self { content: message }
    }
}

impl BlockComponent for MessageDialog {
    fn keymaps(&self) -> &'static [(&'static str, &'static str)] {
        [("any", "dismiss")].as_slice()
    }

    fn on_event(&mut self, _key_event: crossterm::event::KeyEvent) -> HandleResult {
        Ok(HandleSuccess::Consumed)
    }

    fn render(
        &self,
        frame: &mut crate::terminal::Frame,
        area: ratatui::prelude::Rect,
        block: ratatui::widgets::Block,
    ) {
        let (content, title, color) = match &self.content {
            Message::Info(content) => (content.as_str(), " info ", Color::Green),
            Message::Error(content) => (content.as_str(), " error ", Color::Red),
        };

        let p = Paragraph::new(content)
            .block(block.border_style(Style::default().fg(color)).title(title))
            .wrap(Wrap::default());

        frame.render_widget(p, area);
    }
}
