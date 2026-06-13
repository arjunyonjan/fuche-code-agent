use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, DisableLineWrap};
use crossterm::ExecutableCommand;
use std::io::{self, Write};
use colored::*;

struct RawGuard;

impl RawGuard {
    fn enter() -> Self {
        let _ = enable_raw_mode();
        RawGuard
    }
}

impl Drop for RawGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

const ITEMS: &[(&str, &str)] = &[
    ("/help", "Show available commands"),
    ("/mode", "Toggle Build/Plan mode"),
    ("/model", "Show current model"),
    ("/model <name>", "Switch model"),
    ("/models", "List & select from provider models"),
    ("/provider", "Switch provider (ollama/nvidia/clawrouter)"),
    ("/providers", "List available providers"),
    ("/history", "Show last 10 messages"),
    ("/clear", "Clear conversation history"),
    ("/cd <path>", "Change working directory"),
    ("/status", "Show system status"),
    ("/save", "Export conversation to file"),
    ("/exit", "Quit fuchecode"),
];

fn draw(selected: usize) {
    let _ = io::stdout().execute(DisableLineWrap);
    let mut out = String::new();
    out.push_str("\r\n");
    out.push_str(&format!("{}\n", "┌─ Commands ─────────────────────────────────┐".cyan()));
    for (i, (cmd, desc)) in ITEMS.iter().enumerate() {
        let line = if i == selected {
            format!("│ {} {:<10} {}  │\n", "▸".bright_yellow(), cmd.bright_white().bold(), desc.dimmed())
        } else {
            format!("│   {:<10} {}  │\n", cmd.bright_green(), desc.dimmed())
        };
        out.push_str(&line);
    }
    out.push_str(&format!("{}", "└────────────────────────────────────────────┘".cyan()));
    out.push_str("\r\n");
    out.push_str(&format!("\r{}", "  ↑↓ navigate · Enter select · Esc close".dimmed()));
    out.push_str(" \r");
    print!("{}", out);
    let _ = io::stdout().flush();
}

fn clear_popup() {
    // Clear the popup area by moving up and clearing lines
    let height = ITEMS.len() as u16 + 4;
    for _ in 0..height {
        print!("\x1B[2K\r\x1B[1A");
    }
    print!("\r");
    let _ = io::stdout().flush();
}

pub fn show() -> String {
    let _guard = RawGuard::enter();
    let mut selected = 0usize;

    draw(selected);

    loop {
        match event::read() {
            Ok(Event::Key(ke)) if ke.kind == KeyEventKind::Press => {
                match ke.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if selected > 0 {
                            selected -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if selected + 1 < ITEMS.len() {
                            selected += 1;
                        }
                    }
                    KeyCode::Enter => {
                        clear_popup();
                        return ITEMS[selected].0.to_string();
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        clear_popup();
                        return String::new();
                    }
                    _ => {}
                }
                draw(selected);
            }
            Ok(Event::Resize(_, _)) => {
                draw(selected);
            }
            _ => {}
        }
    }
}
