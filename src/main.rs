mod config;
mod ollama;
mod ui;
mod tools;
mod spinner;
mod history;

use std::io::{self, Write};
use colored::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config::Config::load();
    let client = reqwest::Client::new();
    let mut current_model = cfg.default_model.clone();
    
    print!("\x1B[2J\x1B[1;1H");
    ui::header(&current_model);
    
    loop {
        ui::user_prompt();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input == "exit" { break; }
        if input.is_empty() { continue; }
        
        if input == "/model list" {
            let check = client.post("http://172.23.240.1:11434/api/tags")
                .send().await?;
            let tags: serde_json::Value = check.json().await?;
            let models: Vec<String> = tags["models"].as_array().unwrap_or(&vec![])
                .iter()
                .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                .collect();
            println!("\n📦 Available models:");
            for (i, m) in models.iter().enumerate() {
                if m == &current_model {
                    println!("  {} ✅ {}", i+1, m);
                } else {
                    println!("  {}   {}", i+1, m);
                }
            }
            println!("");
            continue;
        }
        
        // Handle /model command
        if input.starts_with("/model ") {
            let new_model = &input[7..];
            if new_model.is_empty() {
                println!("📌 Current model: {}\n", current_model);
                continue;
            }
            
            // Verify model exists in Ollama
            let check = client.post("http://172.23.240.1:11434/api/tags")
                .send().await?;
            let tags: serde_json::Value = check.json().await?;
            let models: Vec<String> = tags["models"].as_array().unwrap_or(&vec![])
                .iter()
                .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                .collect();
            
            if models.contains(&new_model.to_string()) {
                current_model = new_model.to_string();
                println!("✅ Switched to: {}\n", current_model);
                // Refresh header
                print!("\x1B[2J\x1B[1;1H");
                ui::header(&current_model);
            } else {
                println!("❌ Model not found. Available: {}\n", models.join(", "));
            }
            continue;
        }
        
        if input == "/model" {
            println!("📌 Current model: {}\n", current_model);
            continue;
        }
        
        if input == "/history" {
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
            continue;
        }
        
        if input == "/clear" {
            history::clear();
            println!("\n🗑️ History cleared.\n");
            continue;
        }
        
        ui::bot_prefix();
        let response = ollama::chat(&cfg.ollama_url, &current_model, input).await?;
        
        for c in response.chars() {
            print!("{}", c.to_string().bright_green());
            io::stdout().flush()?;
        }
        println!("\n└────────────────────────────────────────────┘\n");
    }
    Ok(())
}
