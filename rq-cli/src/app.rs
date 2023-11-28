use ratatui::{
    prelude::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders},
};
use rq_core::{
    parser::{HttpFile, HttpRequest},
    request::Response,
};
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::components::{
    legend::Legend,
    menu::{self, Menu, MenuItem},
    message_dialog::{self, Message, MessageDialog},
    popup::Popup,
    response_panel::{self, ResponsePanel},
    BlockComponent, HandleSuccess,
};

const KEYMAPS: &[(&str, &str); 1] = &[("Esc/q", "exit")];

#[derive(Default)]
enum FocusState {
    #[default]
    RequestsList,
    ResponseBuffer,
}

impl MenuItem for HttpRequest {
    fn render(&self) -> Vec<ratatui::text::Line<'_>> {
        let mut lines = vec![Line::from(vec![
            Span::styled(self.method.to_string(), Style::default().fg(Color::Green)),
            Span::raw(format!(" {} {:?}", self.url, self.version)),
        ])];

        let headers: Vec<Line> = self
            .headers()
            .iter()
            .map(|(k, v)| {
                Line::from(vec![
                    Span::styled(k.to_string(), Style::default().fg(Color::Blue)),
                    Span::raw(": "),
                    Span::raw(v.to_str().unwrap().to_string()),
                ])
            })
            .collect();
        lines.extend(headers);

        if !self.body.is_empty() {
            lines.push(Line::styled(
                "Focus to show body",
                Style::default()
                    .fg(Color::Rgb(246, 133, 116))
                    .add_modifier(Modifier::ITALIC),
            ));
        }

        lines.push(Line::from(""));
        lines
    }

    fn render_highlighted(&self) -> Vec<Line<'_>> {
        let mut lines = self.render();

        // Underline first line
        lines[0].patch_style(
            Style::default()
                .add_modifier(Modifier::UNDERLINED)
                .add_modifier(Modifier::BOLD),
        );

        // Replace body with expanded version
        if !self.body.is_empty() {
            lines.pop();
            lines.pop();

            for line in self.body.lines() {
                lines.push(Line::styled(
                    line,
                    Style::default().fg(Color::Rgb(246, 133, 116)),
                ));
            }
            lines.push(Line::from(""));
        }

        lines
    }
}

pub struct App {
    res_rx: Receiver<(Response, usize)>,
    req_tx: Sender<(HttpRequest, usize)>,

    request_menu: Menu<HttpRequest>,
    responses: Vec<ResponsePanel>,
    should_exit: bool,
    file_path: String,
    focus: FocusState,
    message_popup: Option<Popup<MessageDialog>>,
}

fn handle_requests(mut req_rx: Receiver<(HttpRequest, usize)>, res_tx: Sender<(Response, usize)>) {
    tokio::spawn(async move {
        while let Some((req, i)) = req_rx.recv().await {
            let data = match rq_core::request::execute(&req).await {
                Ok(data) => data,
                Err(e) => {
                    MessageDialog::push_message(Message::Error(e.to_string()));
                    return;
                }
            };
            res_tx.send((data, i)).await.unwrap();
        }
    });
}

impl App {
    pub fn new(file_path: String, http_file: HttpFile) -> Self {
        let (req_tx, req_rx) = channel::<(HttpRequest, usize)>(1);
        let (res_tx, res_rx) = channel::<(Response, usize)>(1);

        handle_requests(req_rx, res_tx);

        let responses = std::iter::repeat(ResponsePanel::default())
            .take(http_file.requests.len())
            .collect();

        App {
            file_path,
            res_rx,
            req_tx,
            request_menu: Menu::new(http_file.requests),
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

                    self.req_tx
                        .send((
                            self.request_menu.selected().clone(),
                            self.request_menu.idx(),
                        ))
                        .await?;
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
                menu::KEYMAPS.iter(),
            ),
            FocusState::ResponseBuffer => (
                Style::default(),
                Style::default().fg(Color::Blue),
                response_panel::KEYMAPS.iter(),
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
        Legend::new(KEYMAPS.iter().chain(focused_keymaps)).render(
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
                .map(|x| Popup::new(x).with_legend(message_dialog::KEYMAPS.iter()));
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
