use crossterm::event::KeyCode;
use ratatui::widgets::Paragraph;
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::event::Event;

use super::BlockComponent;

type ConfirmCallback = Box<dyn Fn(String)>;
type CancelCallback = Box<dyn Fn()>;

pub struct InputComponent {
    input: Input,
    on_confirm_callback: Option<ConfirmCallback>,
    on_cancel_callback: CancelCallback,
}

impl Default for InputComponent {
    fn default() -> Self {
        Self {
            input: Input::default(),
            on_confirm_callback: None,
            on_cancel_callback: Box::new(|| Event::emit(Event::InputCancel)),
        }
    }
}

impl InputComponent {
    pub fn from(value: &str) -> Self {
        Self {
            input: Input::from(value),
            ..Self::default()
        }
    }

    pub fn with_cursor(self, cursor: usize) -> Self {
        Self {
            input: self.input.with_cursor(cursor),
            ..self
        }
    }

    pub fn with_confirm_callback<F>(self, confirm_callback: F) -> Self
    where
        F: Fn(String) + 'static,
    {
        Self {
            on_confirm_callback: Some(Box::new(confirm_callback)),
            ..self
        }
    }

    pub fn with_cancel_callback<F>(self, cancel_callback: F) -> Self
    where
        F: Fn() + 'static,
    {
        Self {
            on_cancel_callback: Box::new(cancel_callback),
            ..self
        }
    }
}

impl BlockComponent for InputComponent {
    fn keymaps() -> &'static [(&'static str, &'static str)] {
        [("Enter", "confirm"), ("Esc", "cancel")].as_slice()
    }

    fn on_event(&mut self, key_event: crossterm::event::KeyEvent) -> super::HandleResult {
        if self
            .input
            .handle_event(&crossterm::event::Event::Key(key_event))
            .is_some()
        {
            return Ok(super::HandleSuccess::Consumed);
        }

        match key_event.code {
            KeyCode::Enter => {
                if let Some(callback) = self.on_confirm_callback.as_ref() {
                    callback(self.input.value().to_string());
                    return Ok(super::HandleSuccess::Consumed);
                }
            }
            KeyCode::Esc => {
                (self.on_cancel_callback)();
                return Ok(super::HandleSuccess::Consumed);
            }
            _ => (),
        }

        Ok(super::HandleSuccess::Ignored)
    }

    fn render(
        &self,
        frame: &mut crate::terminal::Frame,
        area: ratatui::prelude::Rect,
        block: ratatui::widgets::Block,
    ) {
        let p = Paragraph::new(self.input.value());
        let scroll = self.input.visual_scroll(area.width as usize);

        frame.render_widget(p.block(block), area);
        frame.set_cursor(
            // Put cursor past the end of the input text
            area.x + ((self.input.visual_cursor()).max(scroll) - scroll) as u16 + 1,
            // Move one line down, from the border to the input line
            area.y + 1,
        );
    }
}
