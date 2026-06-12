mod config;
mod ollama;
mod ui;

use std::io::{self, Write};
use colored::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config::Config::new();
    
    print!("\x1B[2J\x1B[1;1H");
    ui::header(&cfg.model);
    
    loop {
        ui::user_prompt();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input == "exit" { break; }
        if input.is_empty() { continue; }
        
        ui::bot_prefix();
        let response = ollama::chat(&cfg.ollama_url, &cfg.model, input).await?;
        
        for c in response.chars() {
            print!("{}", c.to_string().bright_green());
            io::stdout().flush()?;
            std::thread::sleep(std::time::Duration::from_millis(3));
        }
        println!("\n└────────────────────────────────────────────┘\n");
    }
    Ok(())
}
