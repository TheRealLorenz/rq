use crate::{components::response_panel::SaveOption, event::Event};

use super::InputComponent;

pub struct InputBuilder {
    content: String,
    cursor: Option<usize>,
    typ: InputType,
}

pub enum InputType {
    FileName(SaveOption),
    VarValue(String),
}

impl InputBuilder {
    pub fn new(typ: InputType) -> Self {
        Self {
            content: String::new(),
            cursor: None,
            typ,
        }
    }

    pub fn with_content(self, content: String) -> Self {
        Self { content, ..self }
    }

    pub fn with_cursor(self, cursor: usize) -> Self {
        Self {
            cursor: Some(cursor),
            ..self
        }
    }

    fn build_component(&self) -> InputComponent {
        let input = InputComponent::from(&self.content);

        match self.cursor {
            Some(i) => input.with_cursor(i),
            None => input,
        }
    }

    pub fn build(self) -> InputComponent {
        let input = self.build_component();

        match self.typ {
            InputType::FileName(save_option) => input.with_confirm_callback(move |value| {
                Event::emit(Event::InputConfirm);
                Event::emit(Event::Save((value, save_option)));
            }),
            InputType::VarValue(name) => input.with_confirm_callback(move |value| {
                Event::emit(Event::InputConfirm);
                Event::emit(Event::UpdateVar((name.clone(), value)));
            }),
        }
    }
}
