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
        input::InputComponent,
        legend::Legend,
        menu::Menu,
        message_dialog::{Message, MessageDialog},
        popup::Popup,
        response_panel::ResponsePanel,
        vars_panel::VarsPanel,
        BlockComponent, HandleSuccess,
    },
    event::Event,
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

    request_menu: Menu<TemplateRequest>,
    vars_panel: VarsPanel,
    file_path: String,

    responses: Vec<ResponsePanel>,
    should_exit: bool,
    vars_visible: bool,
    focus: FocusState,
    message_popup: Option<Popup<MessageDialog>>,
    input_popup: Option<Popup<InputComponent>>,
}

fn handle_requests(mut req_rx: Receiver<(HttpRequest, usize)>, res_tx: Sender<(Response, usize)>) {
    tokio::spawn(async move {
        while let Some((req, i)) = req_rx.recv().await {
            match rq_core::request::execute(req).await {
                Ok(data) => res_tx.send((data, i)).await.unwrap(),
                Err(e) => MessageDialog::push_message(Message::Error(e.to_string())),
            };
        }
    });
}

impl App {
    const KEYMAPS: &'static [(&'static str, &'static str); 1] = &[("q", "exit")];

    pub fn new(file_path: String, http_file: HttpFile) -> Self {
        let (req_tx, req_rx) = channel::<(HttpRequest, usize)>(1);
        let (res_tx, res_rx) = channel::<(Response, usize)>(1);

        handle_requests(req_rx, res_tx);

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
            message_popup: None,
            input_popup: None,
        }
    }

    async fn on_key_event(&mut self, event: KeyEvent) -> anyhow::Result<()> {
        if let Some(popup) = self.message_popup.as_mut() {
            match popup.on_event(event)? {
                HandleSuccess::Consumed => {
                    self.message_popup = None;
                    return Ok(());
                }
                HandleSuccess::Ignored => (),
            };
        }

        if let Some(popup) = self.input_popup.as_mut() {
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
            Err(e) => {
                MessageDialog::push_message(Message::Error(e.to_string()));
                return Ok(());
            }
        };

        match event.code {
            KeyCode::Char('q' | 'Q') => {
                self.should_exit = true;
            }
            KeyCode::Char('c') => {
                if event.modifiers == KeyModifiers::CONTROL {
                    self.should_exit = true;
                }
            }
            _ => (),
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
        let [mut list_chunk, response_chunk] = {
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
                Legend::new(
                    Self::KEYMAPS
                        .iter()
                        .chain(Menu::<TemplateRequest>::keymaps()),
                ),
            ),
            FocusState::ResponsePanel => (
                Style::default(),
                Style::default().fg(Color::Blue),
                Legend::new(Self::KEYMAPS.iter().chain(ResponsePanel::keymaps())),
            ),
        };

        let list_block = Block::default()
            .borders(Borders::ALL)
            .title(format!(">> {} <<", self.file_path.as_str()))
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

            let var_block = Block::default().borders(Borders::ALL);

            self.vars_panel.render(f, var_chunk, var_block);
        }

        self.request_menu.render(f, list_chunk, list_block);
        let response_panel = &self.responses[self.request_menu.idx()];
        response_panel.render(f, response_chunk, response_block);
        legend.render(f, legend_chunk, Block::default());

        if let Some(popup) = self.message_popup.as_ref() {
            popup.render(f, f.size(), Block::default().borders(Borders::ALL));
        }

        if let Some(popup) = self.input_popup.as_ref() {
            popup.render(f, f.size(), Block::default().borders(Borders::ALL));
        }
    }

    pub fn update(&mut self) {
        // Poll for request responses
        if let Ok((res, i)) = self.res_rx.try_recv() {
            self.responses[i].set_response(res);
        }

        if self.message_popup.is_none() {
            self.message_popup = MessageDialog::pop_message().map(BlockComponent::popup);
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
                        self.input_popup = Some(
                            InputComponent::from(content.as_str())
                                .with_cursor(0)
                                .with_confirm_callback(move |value| {
                                    Event::emit(Event::InputConfirm);
                                    Event::emit(Event::Save((value, save_option)));
                                })
                                .popup(),
                        );
                    }
                };
                Ok(())
            }
            Event::InputCancel => {
                self.input_popup = None;
                Ok(())
            }
            Event::InputConfirm => {
                self.input_popup = None;
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
        };
        if let Err(e) = result {
            MessageDialog::push_message(Message::Error(e.to_string()));
        }
    }
}
