use std::collections::HashMap;

use rq_core::parser::variables::TemplateString;

use crate::components::{menu::Menu, BlockComponent};

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
}
