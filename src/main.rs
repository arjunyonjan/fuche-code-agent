mod config;
mod ollama;
mod ui;
mod tools;
mod spinner;
mod history;
mod commands;
mod mode;

use std::io::{self, Write};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal;
use mode::Mode;

struct RawGuard;
impl RawGuard {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        terminal::enable_raw_mode()?;
        Ok(RawGuard)
    }
}
impl Drop for RawGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config::Config::load();
    let mut current_model = cfg.default_model.clone();
    let mut current_mode = Mode::Build;

    let _guard = RawGuard::new()?;

    print!("\x1B[2J\x1B[1;1H");
    ui::header_with_mode(&current_model, &current_mode);
    ui::user_prompt_with_mode(&current_mode);

    let mut input = String::new();

    loop {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press { continue; }

            match key.code {
                KeyCode::Tab => {
                    current_mode = current_mode.toggle();
                    print!("\x1B[2J\x1B[1;1H");
                    ui::header_with_mode(&current_model, &current_mode);
                    ui::user_prompt_with_mode(&current_mode);
                    if !input.is_empty() {
                        print!("{}", input);
                        io::stdout().flush()?;
                    }
                }
                KeyCode::Enter => {
                    let trimmed = input.trim().to_string();
                    input.clear();
                    print!("\r\n");

                    if trimmed == "exit" { break; }
                    if trimmed.is_empty() {
                        ui::user_prompt_with_mode(&current_mode);
                        continue;
                    }

                    if commands::handle(&trimmed, &mut current_model).await? {
                        ui::user_prompt_with_mode(&current_mode);
                        continue;
                    }

                    ui::bot_prefix_with_mode(&current_mode);
                    let _response = ollama::chat_with_mode(&cfg.ollama_url, &current_model, &trimmed, &current_mode).await?;
                    print!("\r\n");
                    print!("\r└────────────────────────────────────────────┘\n");
                    ui::user_prompt_with_mode(&current_mode);
                }
                KeyCode::Char(c) => {
                    input.push(c);
                    print!("{}", c);
                    io::stdout().flush()?;
                }
                KeyCode::Backspace => {
                    if !input.is_empty() {
                        input.pop();
                        print!("\x08 \x08");
                        io::stdout().flush()?;
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}
