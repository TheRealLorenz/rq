use std::collections::HashMap;

use rq_core::parser::variables::TemplateString;

use super::BlockComponent;

pub struct VarsPanel {
    vars: HashMap<String, TemplateString>,
}

impl VarsPanel {
    pub fn new(vars: HashMap<String, TemplateString>) -> Self {
        Self { vars }
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
        frame.render_widget(block, area);
    }
}
