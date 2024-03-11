use std::collections::VecDeque;
use std::fmt::Write;

use anyhow::anyhow;
use ratatui::{
    prelude::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders},
};
use rq_core::{
    parser::{HttpFile, HttpRequest, TemplateRequest},
    request::Response,
};
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    components::{
        menu::Menu, message_dialog::MessageDialog, popup::Popup, response_panel::ResponsePanel,
        variables::panel::VarsPanel, BlockComponent, HandleSuccess,
    },
    event::{Event, Message},
};

#[derive(Default)]
pub enum FocusState {
    #[default]
    RequestsList,
    ResponsePanel,
    VarsPanel,
}

pub struct App {
    res_rx: Receiver<(Response, usize)>,
    req_tx: Sender<(HttpRequest, usize)>,

    request_menu: Menu<TemplateRequest>,
    vars_panel: VarsPanel,
    file_path: String,

    responses: Vec<ResponsePanel>,
    should_exit: bool,
    vars_visible: bool,
    focus: FocusState,
    popups: VecDeque<Box<dyn BlockComponent>>,
}

fn spawn_request_handler(
    mut req_rx: Receiver<(HttpRequest, usize)>,
    res_tx: Sender<(Response, usize)>,
) {
    tokio::spawn(async move {
        while let Some((req, i)) = req_rx.recv().await {
            match rq_core::request::execute(req).await {
                Ok(data) => res_tx.send((data, i)).await.unwrap(),
                Err(e) => {
                    Event::emit(Event::Message(Message::Error(e.to_string())));
                }
            };
        }
    });
}

impl App {
    const KEYMAPS: &'static [(&'static str, &'static str); 2] =
        &[("q", "exit"), ("v", "variables")];

    pub fn new(file_path: String, http_file: HttpFile) -> Self {
        let (req_tx, req_rx) = channel::<(HttpRequest, usize)>(1);
        let (res_tx, res_rx) = channel::<(Response, usize)>(1);

        spawn_request_handler(req_rx, res_tx);

        let responses = (0..http_file.requests.len())
            .map(|idx| ResponsePanel::default().with_idx(idx))
            .collect();

        let request_menu = Menu::new(http_file.requests)
            .with_confirm_callback(|_| Event::emit(Event::Focus(FocusState::ResponsePanel)));

        App {
            res_rx,
            req_tx,

            request_menu,
            file_path,
            vars_panel: VarsPanel::new(http_file.variables),
            responses,
            should_exit: false,
            vars_visible: true,
            focus: FocusState::default(),
            popups: VecDeque::new(),
        }
    }

    async fn on_key_event(&mut self, event: KeyEvent) -> anyhow::Result<()> {
        if let KeyCode::Char('c') = event.code {
            if event.modifiers == KeyModifiers::CONTROL {
                self.should_exit = true;
                return Ok(());
            }
        }

        if let Some(popup) = self.popups.front_mut() {
            match popup.on_event(event)? {
                HandleSuccess::Consumed => {
                    return Ok(());
                }
                HandleSuccess::Ignored => (),
            };
        }

        // Propagate event to siblings
        let event_result = match self.focus {
            FocusState::RequestsList => self.request_menu.on_event(event),
            FocusState::ResponsePanel => self.responses[self.request_menu.idx()].on_event(event),
            FocusState::VarsPanel => self.vars_panel.on_event(event),
        };

        match event_result {
            Ok(HandleSuccess::Consumed) => {
                return Ok(());
            }
            Ok(HandleSuccess::Ignored) => (),
            Err(e) => return Err(e),
        };

        match event.code {
            KeyCode::Char('q' | 'Q') => {
                self.should_exit = true;
            }
            KeyCode::Char('v') => Event::emit(Event::Focus(FocusState::VarsPanel)),
            KeyCode::Char('?') => Event::emit(Event::Message(Message::Custom(
                "keymaps".into(),
                self.keymaps() + "\nPress any key to close",
            ))),
            _ => (),
        };

        Ok(())
    }

    fn keymaps(&self) -> String {
        let keymaps = match self.focus {
            FocusState::RequestsList => Self::KEYMAPS.iter().chain(self.request_menu.keymaps()),
            FocusState::ResponsePanel => Self::KEYMAPS.iter().chain(self.responses[0].keymaps()),
            FocusState::VarsPanel => Self::KEYMAPS.iter().chain(self.vars_panel.keymaps()),
        };

        keymaps.fold(String::new(), |mut s, (k, v)| {
            let _ = writeln!(s, "{k}: {v}");
            s
        })
    }

    pub fn draw(&self, f: &mut crate::terminal::Frame<'_>) {
        let (list_border_style, response_border_style, vars_border_style) = match self.focus {
            FocusState::RequestsList => (
                Style::default().fg(Color::Blue),
                Style::default(),
                Style::default(),
            ),
            FocusState::ResponsePanel => (
                Style::default(),
                Style::default().fg(Color::Blue),
                Style::default(),
            ),
            FocusState::VarsPanel => (
                Style::default(),
                Style::default(),
                Style::default().fg(Color::Blue),
            ),
        };

        // Create two chunks with equal screen space
        let [mut list_chunk, response_chunk] = {
            let x = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(f.size());

            [x[0], x[1]]
        };

        let list_block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", self.file_path.as_str()))
            .border_style(list_border_style);

        let response_block = Block::default()
            .borders(Borders::ALL)
            .border_style(response_border_style);

        if self.vars_visible {
            let [new_list_chunk, var_chunk] = {
                let x = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                    .split(list_chunk);

                [x[0], x[1]]
            };

            list_chunk = new_list_chunk;

            let var_block = Block::default()
                .borders(Borders::ALL)
                .border_style(vars_border_style);

            self.vars_panel.render(f, var_chunk, var_block);
        }

        self.request_menu.render(f, list_chunk, list_block);
        let response_panel = &self.responses[self.request_menu.idx()];
        response_panel.render(f, response_chunk, response_block);

        if let Some(popup) = self.popups.front() {
            popup.render(f, f.size(), Block::default().borders(Borders::ALL));
        }
    }

    pub fn update(&mut self) {
        // Poll for request responses
        if let Ok((res, i)) = self.res_rx.try_recv() {
            self.responses[i].set_response(res);
        }
    }

    pub fn should_exit(&self) -> bool {
        self.should_exit
    }

    pub async fn on_event(&mut self, e: Event) {
        let result = match e {
            Event::Focus(e) => {
                self.focus = e;
                Ok(())
            }
            Event::Key(e) => self.on_key_event(e).await,
            Event::Other(_) => Ok(()),
            Event::Save((file_name, option)) => match option {
                crate::components::response_panel::SaveOption::All => {
                    self.responses[self.request_menu.idx()].save_all(&file_name)
                }
                crate::components::response_panel::SaveOption::Body => {
                    self.responses[self.request_menu.idx()].save_body(&file_name)
                }
            },
            Event::NewInput(builder) => {
                self.popups.push_back(Box::new(Popup::new(builder.build())));
                Ok(())
            }
            Event::PopupDismiss | Event::InputConfirm | Event::InputCancel => {
                self.popups.pop_front();
                Ok(())
            }
            Event::SendRequest(idx) => {
                self.responses[idx].set_loading();

                match self.request_menu.get(idx).fill(self.vars_panel.vars()) {
                    Ok(request) => self
                        .req_tx
                        .send((request, idx))
                        .await
                        .map_err(|e| anyhow!(e)),

                    Err(e) => Err(anyhow!(e)),
                }
            }
            Event::Message(message) => {
                self.popups
                    .push_back(Box::new(Popup::new(MessageDialog::new(message))));
                Ok(())
            }
            Event::UpdateVar((name, value)) => match value.parse() {
                Ok(value) => {
                    self.vars_panel.update(name, value);
                    Ok(())
                }
                Err(e) => Err(anyhow!(e)),
            },
        };
        if let Err(e) = result {
            Event::emit(Event::Message(Message::Error(e.to_string())));
        }
    }
}
