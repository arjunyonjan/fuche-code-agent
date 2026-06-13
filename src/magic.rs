use ratatui::{
    layout::Alignment,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};
use std::io;
use std::time::Duration;

pub async fn run() {
    let mut terminal = ratatui::init();
    crossterm::terminal::enable_raw_mode().ok();
    crossterm::execute!(io::stdout(), crossterm::event::EnableMouseCapture).ok();

    loop {
        terminal.draw(|f| {
            let area = f.area();
            let block = Block::default()
                .title(" MAGIC MODE ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .style(Style::default().bg(Color::Black));
            let inner = block.inner(area);
            f.render_widget(block, area);

            let text = Paragraph::new("hello world ⚡")
                .style(Style::default().fg(Color::Cyan))
                .alignment(Alignment::Center);
            f.render_widget(text, inner);
        })
        .ok();

        if crossterm::event::poll(Duration::from_millis(100)).ok().unwrap_or(false) {
            if let crossterm::event::Event::Key(k) = crossterm::event::read().ok().unwrap() {
                if k.code == crossterm::event::KeyCode::Esc {
                    break;
                }
            }
        }
    }

    crossterm::execute!(io::stdout(), crossterm::event::DisableMouseCapture).ok();
    crossterm::terminal::disable_raw_mode().ok();
    ratatui::restore();
}
