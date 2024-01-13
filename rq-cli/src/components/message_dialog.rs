use std::{collections::VecDeque, sync::Mutex};

use once_cell::sync::Lazy;
use ratatui::{
    style::{Color, Style},
    widgets::{Paragraph, Wrap},
};

use super::{BlockComponent, HandleResult, HandleSuccess};

static MESSAGES: Lazy<Mutex<VecDeque<Message>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

#[derive(Clone)]
pub enum Message {
    Info(String),
    Error(String),
}

#[derive(Clone)]
pub struct MessageDialog {
    content: Message,
}

impl MessageDialog {
    pub fn push_message(content: Message) {
        MESSAGES.lock().unwrap().push_back(content);
    }

    pub fn pop_message() -> Option<Self> {
        MESSAGES
            .lock()
            .unwrap()
            .pop_front()
            .map(|content| Self { content })
    }
}

impl BlockComponent for MessageDialog {
    fn keymaps() -> impl Iterator<Item = &'static (&'static str, &'static str)> {
        [("any", "dismiss")].iter()
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
