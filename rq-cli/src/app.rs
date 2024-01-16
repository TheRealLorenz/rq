use std::collections::VecDeque;

use anyhow::anyhow;
use ratatui::{
    prelude::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders},
};
use rq_core::{
    parser::{HttpFile, HttpRequest},
    request::Response,
};
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    components::{
        input::InputComponent, legend::Legend, menu::Menu, message_dialog::MessageDialog,
        popup::Popup, response_panel::ResponsePanel, BlockComponent, HandleSuccess,
    },
    event::{Event, Message},
};

#[derive(Default)]
pub enum FocusState {
    #[default]
    RequestsList,
    ResponsePanel,
}

pub struct App {
    res_rx: Receiver<(Response, usize)>,
    req_tx: Sender<(HttpRequest, usize)>,

    request_menu: Menu<HttpRequest>,
    responses: Vec<ResponsePanel>,
    should_exit: bool,
    file_path: String,
    focus: FocusState,
    popups: VecDeque<Popup>,
}

fn spawn_request_handler(
    mut req_rx: Receiver<(HttpRequest, usize)>,
    res_tx: Sender<(Response, usize)>,
) {
    tokio::spawn(async move {
        while let Some((req, i)) = req_rx.recv().await {
            match rq_core::request::execute(&req).await {
                Ok(data) => res_tx.send((data, i)).await.unwrap(),
                Err(e) => {
                    Event::emit(Event::Message(Message::Error(e.to_string())));
                    continue;
                }
            };
        }
    });
}

impl App {
    const KEYMAPS: &'static [(&'static str, &'static str); 1] = &[("q", "exit")];

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
            file_path,
            res_rx,
            req_tx,
            request_menu,
            responses,
            should_exit: false,
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
        };

        match event_result {
            Ok(HandleSuccess::Consumed) => {
                return Ok(());
            }
            Ok(HandleSuccess::Ignored) => (),
            Err(e) => return Err(e),
        };

        if let KeyCode::Char('q' | 'Q') = event.code {
            self.should_exit = true;
        };

        Ok(())
    }

    pub fn draw(&self, f: &mut crate::terminal::Frame<'_>) {
        let [main_chunk, legend_chunk] = {
            let x = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(f.size());

            [x[0], x[1]]
        };

        // Create two chunks with equal screen space
        let [list_chunk, response_chunk] = {
            let x = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_chunk);

            [x[0], x[1]]
        };

        let (list_border_style, response_border_style, legend) = match self.focus {
            FocusState::RequestsList => (
                Style::default().fg(Color::Blue),
                Style::default(),
                Legend::new(Self::KEYMAPS.iter().chain(self.request_menu.keymaps())),
            ),
            FocusState::ResponsePanel => (
                Style::default(),
                Style::default().fg(Color::Blue),
                Legend::new(Self::KEYMAPS.iter().chain(self.responses[0].keymaps())),
            ),
        };

        let list_block = Block::default()
            .borders(Borders::ALL)
            .title(format!(">> {} <<", self.file_path.as_str()))
            .border_style(list_border_style);

        let response_block = Block::default()
            .borders(Borders::ALL)
            .border_style(response_border_style);

        self.request_menu.render(f, list_chunk, list_block);
        let response_panel = &self.responses[self.request_menu.idx()];
        response_panel.render(f, response_chunk, response_block);
        legend.render(f, legend_chunk, Block::default());

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
            Event::NewInput((content, typ)) => {
                match typ {
                    crate::event::InputType::FileName(save_option) => {
                        self.popups.push_back(Popup::new(Box::new(
                            InputComponent::from(content.as_str())
                                .with_cursor(0)
                                .with_confirm_callback(move |value| {
                                    Event::emit(Event::PopupDismiss);
                                    Event::emit(Event::Save((value, save_option)));
                                }),
                        )));
                    }
                };
                Ok(())
            }
            Event::PopupDismiss => {
                self.popups.pop_front();
                Ok(())
            }
            Event::SendRequest(idx) => {
                self.responses[idx].set_loading();

                self.req_tx
                    .send((self.request_menu.get(idx).clone(), idx))
                    .await
                    .map_err(|e| anyhow!(e))
            }
            Event::Message(message) => {
                self.popups
                    .push_back(Popup::new(Box::new(MessageDialog::new(message))));
                Ok(())
            }
        };
        if let Err(e) = result {
            Event::emit(Event::Message(Message::Error(e.to_string())));
        }
    }
}
