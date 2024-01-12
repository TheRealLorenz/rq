use anyhow::anyhow;
use crossterm::event::KeyCode;
use ratatui::{
    prelude::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarState, Wrap},
};
use rq_core::request::{mime::Payload, Response, StatusCode};
use std::{
    fmt::{Display, Write},
    iter,
};
use tui_input::Input;

use crate::{app::FocusState, event::Event};

use super::{
    menu::{Menu, MenuItem},
    message_dialog::{Message, MessageDialog},
    popup::Popup,
    BlockComponent, HandleResult, HandleSuccess,
};

#[derive(Copy, Clone, Default)]
pub enum SaveOption {
    #[default]
    All,
    Body,
}

impl SaveOption {
    fn iterator() -> impl Iterator<Item = SaveOption> {
        [SaveOption::All, SaveOption::Body].iter().copied()
    }
}

impl Display for SaveOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveOption::All => write!(f, "Save entire response"),
            SaveOption::Body => write!(f, "Save response body"),
        }
    }
}

impl MenuItem for SaveOption {
    fn render(&self) -> Vec<Line<'_>> {
        vec![Line::from(self.to_string())]
    }
}

#[derive(Clone, Default)]
enum State {
    #[default]
    Empty,
    Loading,
    Received(Response),
}

#[derive(Clone, Default)]
pub struct ResponsePanel {
    state: State,
    scroll: u16,
    input_popup: Option<Popup<Input>>,
    save_option: SaveOption,
    save_menu: Option<Popup<Menu<SaveOption>>>,
    show_raw: bool,
}

impl ResponsePanel {
    pub const KEYMAPS: &'static [(&'static str, &'static str); 5] = &[
        ("Esc", "back to list"),
        ("↓/↑ j/k", "scroll down/up"),
        ("Enter", "send request"),
        ("s", "save"),
        ("t", "toggle raw bytes"),
    ];

    pub fn set_loading(&mut self) {
        self.state = State::Loading;
    }

    pub fn set_response(&mut self, value: Response) {
        self.state = State::Received(value);
    }
}

impl ResponsePanel {
    fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    fn body(&self) -> anyhow::Result<Payload> {
        match &self.state {
            State::Received(response) => Ok(response.payload.clone()),
            State::Empty | State::Loading => Err(anyhow!("Request not sent")),
        }
    }

    fn to_string(&self) -> anyhow::Result<String> {
        match &self.state {
            State::Received(response) => {
                let headers = response
                    .headers
                    .iter()
                    .fold(String::new(), |mut acc, (k, v)| {
                        writeln!(acc, "{k}: {}", v.to_str().unwrap()).unwrap();
                        acc
                    });

                let body = self.body_as_string().join("\n");

                let s = format!(
                    "{} {}\n{headers}\n\n{body}",
                    response.version, response.status
                );

                Ok(s)
            }
            State::Empty | State::Loading => Err(anyhow!("Request not sent")),
        }
    }

    fn body_as_string(&self) -> Vec<String> {
        match self.body() {
            Ok(body) => match body {
                Payload::Text(t) => iter::once(format!("decoded with encoding '{}':", t.charset))
                    .chain(t.text.lines().map(str::to_string))
                    .collect(),
                Payload::Bytes(b) if self.show_raw => iter::once("lossy utf-8 decode:".to_string())
                    .chain(
                        String::from_utf8_lossy(&b.bytes)
                            .lines()
                            .map(str::to_string),
                    )
                    .collect(),
                Payload::Bytes(_) => vec!["raw bytes".into()],
            },
            Err(e) => vec![e.to_string()],
        }
    }

    fn render_body(&self) -> Vec<Line> {
        let mut lines: Vec<Line> = self.body_as_string().into_iter().map(Line::from).collect();
        lines[0].patch_style(
            Style::default().add_modifier(Modifier::ITALIC.union(Modifier::UNDERLINED)),
        );

        lines
    }
}

impl BlockComponent for ResponsePanel {
    fn on_event(&mut self, key_event: crossterm::event::KeyEvent) -> HandleResult {
        if let Some(input_popup) = self.input_popup.as_mut() {
            match input_popup.on_event(key_event)? {
                HandleSuccess::Consumed => return Ok(HandleSuccess::Consumed),
                HandleSuccess::Ignored => (),
            }

            match key_event.code {
                KeyCode::Enter => {
                    let file_path = input_popup.value().to_string();

                    let to_save = match self.save_option {
                        SaveOption::All => self.to_string()?.into(),
                        SaveOption::Body => match self.body()? {
                            Payload::Bytes(b) => b.bytes,
                            Payload::Text(t) => t.text.into(),
                        },
                    };

                    std::fs::write(&file_path, to_save)?;
                    self.input_popup = None;

                    MessageDialog::push_message(Message::Info(format!("Saved to {file_path}")));

                    return Ok(HandleSuccess::Consumed);
                }
                KeyCode::Esc => {
                    self.input_popup = None;

                    return Ok(HandleSuccess::Consumed);
                }
                _ => (),
            }
        }

        if self.save_menu.is_some() {
            let extension = self
                .body()
                .ok()
                .map(|payload| match payload {
                    Payload::Bytes(b) => b.extension.unwrap_or_default(),
                    Payload::Text(t) => t.extension.unwrap_or_default(),
                })
                .map(|s| ".".to_string() + s.as_str())
                .unwrap_or_default();

            if let Some(menu) = self.save_menu.as_mut() {
                match menu.on_event(key_event)? {
                    HandleSuccess::Consumed => return Ok(HandleSuccess::Consumed),
                    HandleSuccess::Ignored => (),
                }

                match key_event.code {
                    KeyCode::Enter => {
                        self.save_option = *menu.selected();
                        self.save_menu = None;
                        self.input_popup = Some(
                            Popup::new(Input::from(extension).with_cursor(0)).with_legend(
                                iter::once(&("Esc", "close"))
                                    .chain(Menu::<SaveOption>::KEYMAPS.iter()),
                            ),
                        );

                        return Ok(HandleSuccess::Consumed);
                    }
                    KeyCode::Esc => {
                        self.save_menu = None;

                        return Ok(HandleSuccess::Consumed);
                    }
                    _ => (),
                }
            }
        }

        match key_event.code {
            KeyCode::Down | KeyCode::Char('j') => self.scroll_down(),
            KeyCode::Up | KeyCode::Char('k') => self.scroll_up(),
            KeyCode::Char('s') => {
                self.save_menu = Some(
                    Popup::new(Menu::new(SaveOption::iterator().collect())).with_legend(
                        iter::once(&("Esc", "close")).chain(Menu::<SaveOption>::KEYMAPS.iter()),
                    ),
                );
            }
            KeyCode::Char('t') => {
                self.show_raw = !self.show_raw;
            }
            KeyCode::Esc => Event::emit(Event::Focus(FocusState::RequestsList)),
            _ => return Ok(HandleSuccess::Ignored),
        };

        Ok(HandleSuccess::Consumed)
    }

    fn render(
        &self,
        frame: &mut crate::terminal::Frame,
        area: ratatui::prelude::Rect,
        block: ratatui::widgets::Block,
    ) {
        let content = match &self.state {
            State::Received(response) => {
                let mut lines = vec![];

                // First line
                // <VERSION> <STATUS>
                lines.push(Line::from(vec![
                    response.version.clone().into(),
                    " ".into(),
                    Span::styled(
                        response.status.to_string(),
                        Style::default().fg(status_code_color(response.status)),
                    ),
                ]));

                // Headers
                // <KEY>: <VALUE>
                for (k, v) in &response.headers {
                    lines.push(Line::from(vec![
                        Span::styled(format!("{k}"), Style::default().fg(Color::Blue)),
                        ": ".into(),
                        v.to_str().unwrap().into(),
                    ]));
                }

                // Body
                // with initial empty line
                lines.push(Line::from(""));
                lines.append(&mut self.render_body());

                lines
            }
            State::Empty => vec![Line::styled(
                "Empty",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::ITALIC),
            )],
            State::Loading => vec![Line::styled(
                "Loading...",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::ITALIC),
            )],
        };

        let content_length = content
            .iter()
            .map(|line| (line.width() / (block.inner(area).width) as usize) + 1)
            .sum::<usize>();

        let [paragraph_area, scrollbar_area] = {
            let x = Layout::default()
                .direction(ratatui::prelude::Direction::Horizontal)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(block.inner(area));

            [x[0], x[1]]
        };

        let paragraph = Paragraph::new(content)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));

        frame.render_widget(paragraph, paragraph_area);
        frame.render_stateful_widget(
            Scrollbar::default().orientation(ratatui::widgets::ScrollbarOrientation::VerticalRight),
            scrollbar_area,
            &mut ScrollbarState::default()
                .position(self.scroll)
                .content_length(content_length as u16)
                .viewport_content_length(block.inner(area).height),
        );
        frame.render_widget(block, area);

        if let Some(input_popup) = self.input_popup.as_ref() {
            input_popup.render(
                frame,
                frame.size(),
                Block::default()
                    .borders(Borders::ALL)
                    .title(" output path "),
            );
        }

        if let Some(menu) = self.save_menu.as_ref() {
            menu.render(
                frame,
                frame.size(),
                Block::default().borders(Borders::ALL).title(" save menu "),
            );
        }
    }
}

fn status_code_color(status_code: StatusCode) -> Color {
    if status_code.is_success() {
        Color::Green
    } else if status_code.is_redirection() {
        Color::Yellow
    } else if status_code.is_client_error() || status_code.is_server_error() {
        Color::Red
    } else {
        Color::default()
    }
}
