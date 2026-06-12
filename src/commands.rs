use crate::history;
use crate::ui;

pub async fn handle(input: &str, current_model: &mut String) -> Result<bool, Box<dyn std::error::Error>> {
    match input {
        "/help" => {
            println!("\n📚 FUCHECODE Commands\n");
            println!("  /help           - Show this help");
            println!("  /model          - Show current model");
            println!("  /model <name>   - Switch model");
            println!("  /history        - Show last 10 messages");
            println!("  /clear          - Clear conversation history");
            println!("  /status         - Show system status");
            println!("  /save           - Export conversation to file");
            println!("  exit            - Quit fuchecode\n");
            return Ok(true);
        }
        "/status" => {
            let history_count = history::load_last(9999).len();
            println!("\n📊 Status:");
            println!("  Model: {}", current_model);
            println!("  Ollama: Connected");
            println!("  History: {} messages", history_count);
            println!("  Config: ~/.fuchecode/config.toml\n");
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
            println!("\n💾 Exported to: {}\n", filename);
            return Ok(true);
        }
        "/history" => {
            let history = history::load_last(10);
            println!("\n📜 Last 10 messages:");
            for msg in history {
                let role = msg["role"].as_str().unwrap_or("");
                let content = msg["content"].as_str().unwrap_or("");
                if role == "user" {
                    println!("  🧑 You: {}", content);
                } else if role == "assistant" {
                    println!("  🤖 AI: {}", content);
                }
            }
            println!("");
            return Ok(true);
        }
        "/clear" => {
            history::clear();
            println!("\n🗑️ History cleared.\n");
            return Ok(true);
        }
        _ if input.starts_with("/model ") => {
            let new_model = &input[7..];
            if !new_model.is_empty() {
                *current_model = new_model.to_string();
                println!("✅ Switched to: {}\n", current_model);
                print!("\x1B[2J\x1B[1;1H");
                ui::header(current_model);
            }
            return Ok(true);
        }
        "/model" => {
            println!("📌 Current model: {}\n", current_model);
            return Ok(true);
        }
        _ => return Ok(false),
    }
}
