use std::collections::HashMap;

use crossterm::event::KeyCode;
use rq_core::parser::variables::TemplateString;

use crate::{
    components::{menu::Menu, BlockComponent, HandleSuccess},
    event::Event,
};

pub struct VarsPanel {
    vars: HashMap<String, TemplateString>,
    menu: Menu<(String, TemplateString)>,
}

impl VarsPanel {
    pub fn new(vars: HashMap<String, TemplateString>) -> Self {
        Self {
            menu: Menu::new(vars.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
            vars,
        }
    }

    pub fn vars(&self) -> &HashMap<String, TemplateString> {
        &self.vars
    }
}

impl BlockComponent for VarsPanel {
    fn render(
        &self,
        frame: &mut crate::terminal::Frame,
        area: ratatui::prelude::Rect,
        block: ratatui::widgets::Block,
    ) {
        self.menu.render(frame, area, block.title(" Variables "));
    }

    fn on_event(
        &mut self,
        key_event: crossterm::event::KeyEvent,
    ) -> crate::components::HandleResult {
        match self.menu.on_event(key_event)? {
            HandleSuccess::Consumed => return Ok(HandleSuccess::Consumed),
            HandleSuccess::Ignored => (),
        }

        if matches!(key_event.code, KeyCode::Esc) {
            Event::emit(Event::Focus(crate::app::FocusState::RequestsList));
        }

        Ok(HandleSuccess::Ignored)
    }
}
