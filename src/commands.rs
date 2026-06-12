use std::io::{self, Write};
use crate::history;
use crate::ui;

fn rl(s: &str) {
    print!("\r{}\n", s);
    io::stdout().flush().unwrap();
}

pub async fn handle(input: &str, current_model: &mut String) -> Result<bool, Box<dyn std::error::Error>> {
    match input {
        "/help" => {
            rl("");
            rl("📚 FUCHECODE Commands");
            rl("");
            rl("  /help           - Show this help");
            rl("  /model          - Show current model");
            rl("  /model <name>   - Switch model");
            rl("  /history        - Show last 10 messages");
            rl("  /clear          - Clear conversation history");
            rl("  /status         - Show system status");
            rl("  /save           - Export conversation to file");
            rl("  exit            - Quit fuchecode");
            rl("");
            return Ok(true);
        }
        "/status" => {
            let history_count = history::load_last(9999).len();
            rl("");
            rl(&format!("📊 Status:"));
            rl(&format!("  Model: {}", current_model));
            rl("  Ollama: Connected");
            rl(&format!("  History: {} messages", history_count));
            rl("  Config: ~/.fuchecode/config.toml");
            rl("");
            return Ok(true);
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
            return Ok(true);
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
            return Ok(true);
        }
        "/clear" => {
            history::clear();
            rl("");
            rl("🗑️ History cleared.");
            rl("");
            return Ok(true);
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
            return Ok(true);
        }
        "/model" => {
            rl(&format!("📌 Current model: {}", current_model));
            rl("");
            return Ok(true);
        }
        _ => return Ok(false),
    }
}
