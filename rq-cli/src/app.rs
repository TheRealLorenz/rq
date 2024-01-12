use std::collections::HashMap;

use ratatui::{
    prelude::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders},
};
use rq_core::{
    parser::{variables::TemplateString, HttpFile, HttpRequest, TemplateRequest},
    request::Response,
};
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::components::{
    legend::Legend,
    menu::Menu,
    message_dialog::{Message, MessageDialog},
    popup::Popup,
    response_panel::ResponsePanel,
    BlockComponent, HandleSuccess,
};

#[derive(Default)]
enum FocusState {
    #[default]
    RequestsList,
    ResponseBuffer,
}

pub struct App {
    res_rx: Receiver<(Response, usize)>,
    req_tx: Sender<(HttpRequest, usize)>,

    request_menu: Menu<TemplateRequest>,
    variables: HashMap<String, TemplateString>,
    file_path: String,

    responses: Vec<ResponsePanel>,
    should_exit: bool,
    focus: FocusState,
    message_popup: Option<Popup<MessageDialog>>,
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
    const KEYMAPS: &'static [(&'static str, &'static str); 1] = &[("Esc/q", "exit")];

    pub fn new(file_path: String, http_file: HttpFile) -> Self {
        let (req_tx, req_rx) = channel::<(HttpRequest, usize)>(1);
        let (res_tx, res_rx) = channel::<(Response, usize)>(1);

        handle_requests(req_rx, res_tx);

        let responses = std::iter::repeat(ResponsePanel::default())
            .take(http_file.requests.len())
            .collect();

        App {
            res_rx,
            req_tx,

            request_menu: Menu::new(http_file.requests),
            variables: http_file.variables,
            file_path,

            responses,
            should_exit: false,
            focus: FocusState::default(),
            message_popup: None,
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

        // Propagate event to siblings
        let event_result = match self.focus {
            FocusState::RequestsList => self.request_menu.on_event(event),
            FocusState::ResponseBuffer => self.responses[self.request_menu.idx()].on_event(event),
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
            KeyCode::Esc if matches!(self.focus, FocusState::ResponseBuffer) => {
                self.focus = FocusState::RequestsList;
            }
            KeyCode::Enter => match self.focus {
                FocusState::RequestsList => self.focus = FocusState::ResponseBuffer,
                FocusState::ResponseBuffer => {
                    let index = self.request_menu.idx();
                    self.responses[index].set_loading();

                    match self.request_menu.selected().fill(&self.variables) {
                        Ok(request) => self.req_tx.send((request, self.request_menu.idx())).await?,
                        Err(e) => MessageDialog::push_message(Message::Error(e.to_string())),
                    };
                }
            },
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
        let [list_chunk, response_chunk] = {
            let x = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_chunk);

            [x[0], x[1]]
        };

        let (list_border_style, response_border_style, focused_keymaps) = match self.focus {
            FocusState::RequestsList => (
                Style::default().fg(Color::Blue),
                Style::default(),
                Menu::<TemplateRequest>::KEYMAPS.iter(),
            ),
            FocusState::ResponseBuffer => (
                Style::default(),
                Style::default().fg(Color::Blue),
                ResponsePanel::KEYMAPS.iter(),
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
        Legend::new(Self::KEYMAPS.iter().chain(focused_keymaps)).render(
            f,
            legend_chunk,
            Block::default(),
        );

        if let Some(popup) = self.message_popup.as_ref() {
            popup.render(f, f.size(), Block::default().borders(Borders::ALL));
        }
    }

    pub fn update(&mut self) {
        // Poll for request responses
        if let Ok((res, i)) = self.res_rx.try_recv() {
            self.responses[i].set_response(res);
        }

        if self.message_popup.is_none() {
            self.message_popup = MessageDialog::pop_message()
                .map(|x| Popup::new(x).with_legend(MessageDialog::KEYMAPS.iter()));
        }
    }

    pub fn should_exit(&self) -> bool {
        self.should_exit
    }

    pub async fn on_event(&mut self, e: crossterm::event::Event) -> anyhow::Result<()> {
        if let Event::Key(e) = e {
            self.on_key_event(e).await?;
        }
        Ok(())
    }
}
