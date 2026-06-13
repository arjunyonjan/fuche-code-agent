use std::io::{self, Write};
use colored::*;
use crate::history;
use crate::ui;
use crate::ollama;
use crate::config::Config;

fn rl(s: &str) {
    print!("\r{}\n", s);
    let _ = io::stdout().flush();
}

pub async fn handle(input: &str, current_model: &mut String, cfg: &mut Config) -> Result<bool, Box<dyn std::error::Error>> {
    match input {
        "/help" => {
            rl("");
            rl("📚 FUCHECODE Commands");
            rl("");
            rl("  /help           - Show this help");
            rl("  /model          - Show current model");
            rl("  /model <name>   - Switch model");
            rl("  /models         - List & select from provider models");
            rl("  /provider       - Switch provider (ollama/nvidia/clawrouter)");
            rl("  /providers      - List available providers");
            rl("  /history        - Show last 10 messages");
            rl("  /clear          - Clear conversation history");
            rl("  /cd <path>      - Change working directory");
            rl("  /status         - Show system status");
            rl("  /save           - Export conversation to file");
            rl("  exit            - Quit fuchecode");
            rl("");
            Ok(true)
        }
        "/status" => {
            let history_count = history::load_last(9999).len();
            rl("");
            rl("📊 Status:");
            rl(&format!("  Provider: {}", cfg.current_provider));
            rl(&format!("  Model: {}", current_model));
            rl(&format!("  API: {}", cfg.api_url()));
            rl(&format!("  Timeout: {}s", cfg.timeout_secs));
            rl(&format!("  History: {} messages", history_count));
			rl("  Config: ~/.fuchecode/config.toml");
			if let Ok(cwd) = std::env::current_dir() {
				rl(&format!("  CWD: {}", cwd.display()));
			}
			rl("");
            Ok(true)
        }
        "/save" => {
            let history = history::load_last(9999);
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            let filename = format!("/home/arjun/fuchecode_export_{}.md", timestamp);
            let mut content = String::from("# FUCHECODE Conversation Export\n\n");
            for msg in history {
                let role = msg["role"].as_str().unwrap_or("");
                let text = msg["content"].as_str().unwrap_or("");
                if role == "user" {
                    content.push_str(&format!("## 👤 User\n{}\n\n", text));
                } else if role == "assistant" {
                    content.push_str(&format!("## 🤖 Assistant\n{}\n\n", text));
                }
            }
            std::fs::write(&filename, content)?;
            rl("");
            rl(&format!("💾 Exported to: {}", filename));
            rl("");
            Ok(true)
        }
        "/history" => {
            rl("");
            rl("📜 Last 10 messages:");
            for msg in history::load_last(10) {
                let role = msg["role"].as_str().unwrap_or("");
                let content = msg["content"].as_str().unwrap_or("");
                if role == "user" {
                    rl(&format!("  🧑 You: {}", content));
                } else if role == "assistant" {
                    rl(&format!("  🤖 AI: {}", content));
                }
            }
            rl("");
            Ok(true)
        }
        "/clear" => {
            history::clear();
            rl("");
            rl("🗑️ History cleared.");
            rl("");
            Ok(true)
        }
        "/cd" => {
            match std::env::current_dir() {
                Ok(p) => { rl(&format!("📂 Current: {}", p.display())); rl(""); }
                Err(_) => { rl("❌ Cannot get CWD"); rl(""); }
            }
            Ok(true)
        }
        _ if input.starts_with("/cd ") => {
            let raw = input[4..].trim();
            let path = if raw.starts_with("~/") || raw == "~" {
                let home = std::env::var("HOME").unwrap_or_default();
                format!("{}{}", home, &raw[1..])
            } else {
                raw.to_string()
            };
            match std::env::set_current_dir(&path) {
                Ok(_) => { rl(&format!("✅ CWD changed to: {}", path)); rl(""); }
                Err(e) => { rl(&format!("❌ Error: {}", e)); rl(""); }
            }
            Ok(true)
        }
        "/providers" => {
            rl("");
            rl("📡 Available providers:");
            for name in cfg.provider_names() {
                let marker = if name == cfg.current_provider { "➡️ " } else { "   " };
                let url = cfg.providers.get(&name).map(|p| p.url.as_str()).unwrap_or("");
                rl(&format!("  {}{}  ({})", marker, name.cyan(), url.dimmed()));
            }
            rl("");
            rl("  Use /provider <name> to switch");
            rl("");
            Ok(true)
        }
        "/provider" => {
            rl(&format!("📡 Current provider: {}", cfg.current_provider.cyan()));
            rl("");
            Ok(true)
        }
        _ if input.starts_with("/provider ") => {
            let name = &input[10..];
            if cfg.providers.contains_key(name) {
                cfg.current_provider = name.to_string();
                cfg.save();
                rl(&format!("✅ Switched provider to: {}", name));
                rl(&format!("   API: {}", cfg.api_url()));

                if name == "clawrouter" {
                    let online = crate::ollama::check_cr_status().await;
                    if online {
                        rl("   🟢 ClawRouter active at :8402");
                    } else {
                        rl("   🔴 ClawRouter offline — run: npx @blockrun/clawrouter@latest");
                    }
                }

                rl("");
                print!("\x1B[2J\x1B[1;1H");
                ui::header(current_model);
            } else {
                rl(&format!("❌ Unknown provider: {}", name));
                rl(&format!("   Available: {}", cfg.provider_names().join(", ")));
                rl("");
            }
            Ok(true)
        }
        _ if input.starts_with("/model ") => {
            let new_model = &input[7..];
            if !new_model.is_empty() {
                *current_model = new_model.to_string();
                rl(&format!("✅ Switched to: {}", current_model));
                rl("");
                print!("\x1B[2J\x1B[1;1H");
                ui::header(current_model);
            }
            Ok(true)
        }
        "/model" => {
            rl(&format!("📌 Current model: {}", current_model));
            rl("");
            Ok(true)
        }
        "/models" | "/models all" => {
            let show_all = input == "/models all";
            let api_url = cfg.api_url();
            rl("");
            rl("📋 Fetching available models...");
            match ollama::list_models(&api_url, cfg.timeout_secs).await {
                Ok(models) => {
                    if models.is_empty() {
                        rl("  No models found.");
                        rl("");
                        return Ok(true);
                    }

                    let display: Vec<&String> = if show_all {
                        models.iter().collect()
                    } else {
                        models.iter().filter(|m| m.starts_with("free/")).collect()
                    };

                    if display.is_empty() {
                        rl(&format!("  No free models found. Use /models all to see all {} models.", models.len()));
                        rl("");
                        return Ok(true);
                    }

                    rl("");
                    let total = if show_all { format!(" — {} total", models.len()) } else { format!(" — {} free ({} total, use /models all for full)", display.len(), models.len()) };
                    rl(&format!("{}", format!("┌─ Models{} ────────────────┐", total).cyan()));
                    for (i, name) in display.iter().enumerate() {
                        rl(&format!("│ {:<3} {}  │", format!("{}.", i + 1).bright_green(), name));
                    }
                    rl(&format!("{}", "└────────────────────────────────────────────┘".cyan()));
                    rl("");

                    rl("Enter model number to switch, or 0 to cancel:");
                    rl("");
                    ui::user_prompt_with_mode(&crate::mode::Mode::Build);

                    let mut sel = String::new();
                    io::stdin().read_line(&mut sel)?;
                    let sel = sel.trim();

                    if let Ok(n) = sel.parse::<usize>() {
                        if n > 0 && n <= display.len() {
                            let chosen = &display[n - 1];
                            *current_model = (*chosen).clone();
                            rl(&format!("✅ Switched to: {}", current_model));
                            rl("");
                            print!("\x1B[2J\x1B[1;1H");
                            ui::header(current_model);
                        } else if n == 0 {
                            rl("Canceled.");
                            rl("");
                        } else {
                            rl("Invalid selection.");
                            rl("");
                        }
                    } else {
                        rl("Invalid input.");
                        rl("");
                    }
                }
                Err(e) => {
                    rl(&format!("❌ Failed to fetch models: {}", e));
                    rl("");
                }
            }
            Ok(true)
        }
        "/magic" => {
            crate::magic::run().await;
            Ok(true)
        }
        _ => Ok(false),
    }
}
