use ratatui::{
    style::{Color, Style},
    widgets::{Paragraph, Wrap},
};

use crate::event::{Event, Message};

use super::{BlockComponent, HandleResult, HandleSuccess};

pub struct MessageDialog {
    content: Message,
}
impl MessageDialog {
    pub fn new(message: Message) -> Self {
        Self { content: message }
    }

    fn format_title(title: &str) -> String {
        format!(" {title} ")
    }
}

impl BlockComponent for MessageDialog {
    fn keymaps(&self) -> &'static [(&'static str, &'static str)] {
        [("any", "dismiss")].as_slice()
    }

    fn on_event(&mut self, _key_event: crossterm::event::KeyEvent) -> HandleResult {
        Event::emit(Event::PopupDismiss);

        Ok(HandleSuccess::Consumed)
    }

    fn render(
        &self,
        frame: &mut crate::terminal::Frame,
        area: ratatui::prelude::Rect,
        block: ratatui::widgets::Block,
    ) {
        let (content, title, color) = match &self.content {
            Message::Info(content) => (content.as_str(), Self::format_title("info"), Color::Green),
            Message::Error(content) => (content.as_str(), Self::format_title("error"), Color::Red),
            Message::Custom(title, content) => {
                (content.as_str(), Self::format_title(title), Color::Green)
            }
        };

        let p = Paragraph::new(content)
            .block(block.border_style(Style::default().fg(color)).title(title))
            .wrap(Wrap::default());

        frame.render_widget(p, area);
    }
}
