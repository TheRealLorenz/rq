use crossterm::event::KeyCode;
use ratatui::{
    text::Line,
    widgets::{List, ListItem, ListState},
};

use super::BlockComponent;

pub trait MenuItem {
    fn render(&self) -> Vec<Line<'_>>;
    fn render_highlighted(&self) -> Vec<Line<'_>> {
        self.render()
    }
}

type ConfirmCallback<T> = Box<dyn Fn(&T)>;

pub struct Menu<T: MenuItem> {
    idx: usize,
    items: Vec<T>,
    on_confirm_callback: Option<ConfirmCallback<T>>,
}

impl<T: MenuItem> Menu<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self {
            idx: 0,
            items,
            on_confirm_callback: None,
        }
    }

    fn next(&mut self) {
        self.idx = (self.idx + 1) % self.items.len();
    }

    fn previous(&mut self) {
        self.idx = match self.idx {
            0 => self.items.len() - 1,
            i => i - 1,
        };
    }

    pub fn selected(&self) -> &T {
        &self.items[self.idx]
    }

    pub fn idx(&self) -> usize {
        self.idx
    }

    pub fn get(&self, idx: usize) -> &T {
        &self.items[idx]
    }

    pub fn with_confirm_callback<F>(self, confirm_callback: F) -> Self
    where
        F: Fn(&T) + 'static,
    {
        Self {
            on_confirm_callback: Some(Box::new(confirm_callback)),
            ..self
        }
    }
}

impl<T: MenuItem> BlockComponent for Menu<T> {
    fn keymaps() -> impl Iterator<Item = &'static (&'static str, &'static str)> {
        [("↓/↑ j/k", "next/previous"), ("Enter", "select")].iter()
    }

    fn on_event(&mut self, key_event: crossterm::event::KeyEvent) -> super::HandleResult {
        match key_event.code {
            KeyCode::Char('j') | KeyCode::Down => self.next(),
            KeyCode::Char('k') | KeyCode::Up => self.previous(),
            KeyCode::Enter => {
                if let Some(callback) = self.on_confirm_callback.as_ref() {
                    callback(self.selected());
                }
            }
            _ => return Ok(super::HandleSuccess::Ignored),
        }

        Ok(super::HandleSuccess::Consumed)
    }

    fn render(
        &self,
        frame: &mut crate::terminal::Frame,
        area: ratatui::prelude::Rect,
        block: ratatui::widgets::Block,
    ) {
        let items = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                if self.idx == i {
                    ListItem::new(item.render_highlighted())
                } else {
                    ListItem::new(item.render())
                }
            })
            .collect::<Vec<_>>();

        let list = List::new(items).highlight_symbol("> ");

        frame.render_stateful_widget(
            list.block(block),
            area,
            &mut ListState::default().with_selected(Some(self.idx)),
        );
    }
}
