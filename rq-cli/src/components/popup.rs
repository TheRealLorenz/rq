use ratatui::{
    prelude::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Clear},
};
use std::ops::{Deref, DerefMut};

use super::{legend::Legend, BlockComponent};

pub struct Popup<T: BlockComponent> {
    component: T,
    w_percent: u16,
    h_percent: u16,
}

impl<T: BlockComponent> Popup<T> {
    pub fn new(widget: T) -> Self {
        Self {
            component: widget,
            w_percent: 40,
            h_percent: 25,
        }
    }
}

impl<T: BlockComponent> Deref for Popup<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.component
    }
}

impl<T: BlockComponent> DerefMut for Popup<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.component
    }
}

impl<T: BlockComponent> BlockComponent for Popup<T> {
    fn on_event(&mut self, key_event: crossterm::event::KeyEvent) -> super::HandleResult {
        self.component.on_event(key_event)
    }

    fn update(&mut self) {
        self.component.update();
    }

    fn render(
        &self,
        frame: &mut crate::terminal::Frame,
        area: Rect,
        block: ratatui::widgets::Block,
    ) {
        let popup_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - self.w_percent) / 2),
                Constraint::Percentage(self.w_percent),
                Constraint::Percentage((100 - self.w_percent) / 2),
            ])
            .split(area)[1];
        let popup_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - self.h_percent) / 2),
                Constraint::Percentage(self.h_percent),
                Constraint::Percentage((100 - self.h_percent) / 2),
            ])
            .split(popup_area)[1];

        frame.render_widget(Clear, popup_area);

        let [popup_area, legend_area] = {
            let x = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(popup_area);

            [x[0], x[1]]
        };

        self.component.render(frame, popup_area, block);
        Legend::new(T::keymaps()).render(frame, legend_area, Block::default());
    }
}
