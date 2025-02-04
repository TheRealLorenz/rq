use crossterm::event::KeyEvent;
use ratatui::{prelude::Rect, widgets::Block};

use crate::terminal::Frame;

pub mod input;
pub mod menu;
pub mod message_dialog;
pub mod popup;
pub mod response_panel;
pub mod template_request;
pub mod variables;

pub enum HandleSuccess {
    Consumed,
    Ignored,
}

pub type HandleResult = anyhow::Result<HandleSuccess>;

pub trait Component {
    fn on_event(&mut self, _key_event: KeyEvent) -> HandleResult {
        Ok(HandleSuccess::Ignored)
    }
    fn update(&mut self) {}
    fn render(&self, frame: &mut Frame, area: Rect);
}

pub trait BlockComponent {
    fn on_event(&mut self, _key_event: KeyEvent) -> HandleResult {
        Ok(HandleSuccess::Ignored)
    }
    fn update(&mut self) {}
    fn render(&self, frame: &mut Frame, area: Rect, block: Block);
    fn keymaps(&self) -> &'static [(&'static str, &'static str)] {
        &[]
    }
}
