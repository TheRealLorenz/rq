use crossterm::{
    event, execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::time::Duration;

use crate::{app::App, event::Event};

pub type Frame<'a> = ratatui::Frame<'a, CrosstermBackend<std::io::Stderr>>;

fn startup() -> std::io::Result<()> {
    enable_raw_mode()?;
    execute!(std::io::stderr(), EnterAlternateScreen)?;
    Ok(())
}

fn shutdown() -> std::io::Result<()> {
    execute!(std::io::stderr(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

async fn main_loop(app: &mut App) -> anyhow::Result<()> {
    let mut t = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;

    loop {
        app.update();

        if event::poll(Duration::from_millis(250))? {
            app.on_event(Event::parse(event::read()?)).await?;
        }

        if let Some(event) = Event::poll() {
            app.on_event(event).await?;
        }

        t.draw(|f| {
            app.draw(f);
        })?;

        if app.should_exit() {
            break;
        }
    }

    Ok(())
}

pub async fn run(mut app: App) -> anyhow::Result<()> {
    startup()?;
    let res = main_loop(&mut app).await;
    shutdown()?;

    res?;

    Ok(())
}
