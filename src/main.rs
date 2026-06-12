mod config;
mod ollama;
mod ui;
mod tools;
mod spinner;
mod history;
mod commands;

use std::io::{self, Write};
use colored::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config::Config::load();
    let mut current_model = cfg.default_model.clone();
    
    print!("\x1B[2J\x1B[1;1H");
    ui::header(&current_model);
    
    loop {
        ui::user_prompt();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input == "exit" || input == "/exit" { break; }
        if input.is_empty() { continue; }
        
        // Handle commands
        if commands::handle(input, &mut current_model).await? {
            continue;
        }
        
        // AI Chat
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
