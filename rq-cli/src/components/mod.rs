use crossterm::event::KeyEvent;
use ratatui::{prelude::Rect, widgets::Block};

use crate::terminal::Frame;

use self::popup::Popup;

pub mod input;
pub mod legend;
pub mod menu;
pub mod message_dialog;
pub mod popup;
pub mod response_panel;
pub mod template_request;
pub mod vars_panel;

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
    fn popup(self) -> Popup<Self>
    where
        Self: std::marker::Sized,
    {
        Popup::new(self)
    }
    fn keymaps() -> impl Iterator<Item = &'static (&'static str, &'static str)> {
        std::iter::empty()
    }
}
