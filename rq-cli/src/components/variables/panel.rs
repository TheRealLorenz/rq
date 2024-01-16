use std::collections::HashMap;

use crossterm::event::KeyCode;
use rq_core::parser::variables::TemplateString;

use crate::{
    components::{menu::Menu, BlockComponent, HandleSuccess},
    event::{Event, InputType},
};

pub struct VarsPanel {
    vars: HashMap<String, TemplateString>,
    menu: Menu<(String, TemplateString)>,
}

impl VarsPanel {
    pub fn new(vars: HashMap<String, TemplateString>) -> Self {
        let menu = Menu::new(vars.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .with_confirm_callback(|(name, value)| {
                Event::emit(Event::NewInput((
                    value.to_string(),
                    InputType::VarValue(name.clone()),
                )));
            });

        Self { vars, menu }
    }

    pub fn vars(&self) -> &HashMap<String, TemplateString> {
        &self.vars
    }

    pub fn update(&mut self, name: String, value: TemplateString) {
        match self.vars.insert(name.clone(), value.clone()) {
            Some(_) => {
                let cloned = name.clone();
                self.menu.update(move |(n, _)| n == &cloned, (name, value));
            }
            None => self.menu.add((name, value)),
        };
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

    fn keymaps() -> impl Iterator<Item = &'static (&'static str, &'static str)> {
        std::iter::once(&("Esc", "back to list")).chain(Menu::<(String, TemplateString)>::keymaps())
    }
}
