use std::{collections::VecDeque, sync::Mutex};

use once_cell::sync::Lazy;

use crate::{app::FocusState, components::response_panel::SaveOption};

static EVENT_QUEUE: Lazy<Mutex<VecDeque<Event>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

pub enum Event {
    Focus(FocusState),
    Save((String, SaveOption)),
    NewInput((String, InputType)),
    InputConfirm,
    InputCancel,
    SendRequest(usize),
    Message(Message),
    Key(crossterm::event::KeyEvent),
    Other(crossterm::event::Event),
}

pub enum InputType {
    FileName(SaveOption),
}

pub enum Message {
    Info(String),
    Error(String),
}

impl Event {
    pub fn emit(event: Event) {
        EVENT_QUEUE.lock().unwrap().push_front(event);
    }

    pub fn poll() -> Option<Self> {
        EVENT_QUEUE.lock().unwrap().pop_back()
    }

    pub fn parse(event: crossterm::event::Event) -> Self {
        match event {
            crossterm::event::Event::Key(e) => Self::Key(e),
            _ => Self::Other(event),
        }
    }
}
