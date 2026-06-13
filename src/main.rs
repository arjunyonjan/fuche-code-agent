mod config;
mod ollama;
mod ui;
mod mode;
mod spinner;
mod history;
mod commands;
mod tools;
mod popup;
mod magic;
mod mapper;

use std::io::{self, Write};
use colored::*;
use mode::Mode;
use tokio::select;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cfg = config::Config::load();
    let mut current_model = cfg.default_model.clone();
    let mut current_mode = Mode::Build;

    print!("\x1B[2J\x1B[1;1H");
    ui::header_with_mode(&current_model, &current_mode);
    let cr_online = if cfg.current_provider == "clawrouter" {
        ollama::check_cr_status().await
    } else {
        false
    };
    ui::show_guide(&cfg.current_provider, cr_online);

    loop {
        ui::user_prompt_with_mode(&current_mode);

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input == "exit" || input == "/exit" { break; }
        if input.is_empty() { continue; }

        if input == "/" {
            let cmd = popup::show();
            if cmd.is_empty() { continue; }
            if cmd == "/exit" { break; }
            if cmd == "/mode" {
                current_mode = current_mode.toggle();
                print!("\x1B[2J\x1B[1;1H");
                ui::header_with_mode(&current_model, &current_mode);
                continue;
            }
            if commands::handle(&cmd, &mut current_model, &mut cfg).await? {
                continue;
            }
            continue;
        }

        if input == "tab" || input == "/mode" {
            current_mode = current_mode.toggle();
            print!("\x1B[2J\x1B[1;1H");
            ui::header_with_mode(&current_model, &current_mode);
            continue;
        }

        if commands::handle(input, &mut current_model, &mut cfg).await? {
            continue;
        }

        let api_url = cfg.api_url();

        match mapper::map(input) {
            mapper::Action::Execute(cmd) => {
                ui::bot_prefix_with_mode(&current_mode);
                let perms = current_mode.permissions();
                let result = tools::run_command(&cmd, &perms);
                if result.starts_with("⛔ ") {
                    println!("{}", result);
                    println!("\n└────────────────────────────────────────────┘\n");
                    continue;
                }
                let resp = ollama::single_turn(&api_url, &current_model, input, &cmd, &result, cfg.timeout_secs).await;
                println!("{}", resp.bright_green());
                println!("\n└────────────────────────────────────────────┘\n");
                continue;
            }
            mapper::Action::PassThrough => {}
        }

        let force_tool = input.starts_with("open ") || input.starts_with("add ") || input.starts_with("update ")
            || input.starts_with("edit ") || input.starts_with("change ") || input.starts_with("modify ")
            || input.starts_with("create ") || input.starts_with("write ") || input.starts_with("make ")
            || input.starts_with("ls") || input.starts_with("cat ") || input.starts_with("echo ")
            || input.starts_with("pwd") || input.starts_with("whoami") || input.starts_with("which ")
            || input.starts_with("mkdir ") || input.starts_with("rm ") || input.starts_with("cp ")
            || input.starts_with("mv ") || input.starts_with("grep ") || input.starts_with("curl ")
            || input.starts_with("git ") || input.starts_with("cd ") || input.starts_with("chmod ")
            || input.starts_with("npx ") || input.starts_with("npm ") || input.starts_with("cargo ")
            || input.starts_with("docker ") || input.starts_with("ping ") || input.starts_with("wget ")
            || input.starts_with("ps ") || input.starts_with("top ") || input.starts_with("df ")
            || input.starts_with("du ") || input.starts_with("uname ") || input.starts_with("env ")
            || input.starts_with("printenv ") || input.starts_with("alias ") || input.starts_with("type ")
            || input.starts_with("id ") || input.starts_with("who ") || input.starts_with("wc ")
            || input.starts_with("sort ") || input.starts_with("head ") || input.starts_with("tail ")
            || input.starts_with("find ") || input.starts_with("locate ") || input.starts_with("tree ");
        ui::bot_prefix_with_mode(&current_mode);
        let spinner = spinner::start_spinner();

        let (mut rx, _chat_handle) = ollama::chat_stream(&api_url, &current_model, input, cfg.timeout_secs, current_mode, force_tool);
        let mut response = String::new();
        let mut cancelled = false;
        let mut got_token = false;

        loop {
            select! {
                token = rx.recv() => {
                    match token {
                        Some(t) => {
                            if !got_token {
                                got_token = true;
                                spinner.stop();
                            }
                            if response.is_empty() {
                                let trimmed = t.trim_start().to_string();
                                response.push_str(&trimmed);
                                for c in trimmed.chars() {
                                    print!("{}", c.to_string().bright_green());
                                    io::stdout().flush()?;
                                    tokio::time::sleep(std::time::Duration::from_millis(8)).await;
                                }
                            } else {
                                response.push_str(&t);
                                for c in t.chars() {
                                    print!("{}", c.to_string().bright_green());
                                    io::stdout().flush()?;
                                    tokio::time::sleep(std::time::Duration::from_millis(8)).await;
                                }
                            }
                        }
                        None => break,
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    cancelled = true;
                    break;
                }
            }
        }

        if cancelled {
            spinner.stop();
            println!("\r⚠️ Generation cancelled");
            continue;
        }

        if !got_token {
            spinner.stop();
            println!("\r⚠️ No response from API (check /status for provider/model)");
            continue;
        }

        if response.starts_with("❌ ") {
            history::save(input, &response);
            println!("\n");
            continue;
        }
        history::save(input, &response);
        println!("\n└────────────────────────────────────────────┘\n");
    }
    Ok(())
}

#[cfg(test)]
mod user_tests {
    use crate::{mapper, tools, mode::Mode};

    #[test]
    fn test_user_runs_echo() {
        let input = "run echo hello";
        match mapper::map(input) {
            mapper::Action::Execute(cmd) => {
                let result = tools::run_command(&cmd, &Mode::Build.permissions());
                assert_eq!(result, "hello");
            }
            mapper::Action::PassThrough => panic!("Expected Execute, got PassThrough"),
        }
    }

    #[test]
    fn test_user_adds_text_to_file() {
        let path = "/tmp/fuchecode_user_test_add.txt";
        let _ = std::fs::remove_file(path);
        let input = format!("add hello world to {}", path);

        match mapper::map(&input) {
            mapper::Action::Execute(cmd) => {
                let result = tools::run_command(&cmd, &Mode::Build.permissions());
                assert!(result == "✅ Done." || result.contains("hello"));
                let content = std::fs::read_to_string(path).unwrap_or_default();
                assert!(content.contains("hello world"));
            }
            mapper::Action::PassThrough => panic!("Expected Execute, got PassThrough"),
        }

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_user_dangerous_command_blocked() {
        let input = "run sudo rm -rf /";
        match mapper::map(input) {
            mapper::Action::Execute(cmd) => {
                let result = tools::run_command(&cmd, &Mode::Build.permissions());
                assert!(result.starts_with("⛔"), "dangerous command should be blocked: {}", result);
            }
            mapper::Action::PassThrough => panic!("Expected Execute, got PassThrough"),
        }
    }
}
