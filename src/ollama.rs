use reqwest;
use serde_json::{json, Value};
use std::fs;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;
use std::process::Command;

fn chirp() {
    let _ = Command::new("powershell.exe")
        .args(&["-c", "[System.Console]::Beep(1000, 100)"])
        .output();
}

pub async fn chat(url: &str, model: &str, prompt: &str) -> Result<String, reqwest::Error> {
    let client = reqwest::Client::new();
    
    let res = client.post(url)
        .json(&json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "stream": false,
            "tools": [{
                "type": "function",
                "function": {
                    "name": "read_file",
                    "description": "Read a file",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string"}
                        }
                    }
                }
            }]
        }))
        .send().await?;
    
    let mut json: Value = res.json().await?;
    
    if let Some(tool_calls) = json["message"]["tool_calls"].as_array() {
        if !tool_calls.is_empty() {
            let tool = &tool_calls[0];
            let path = tool["function"]["arguments"]["path"].as_str().unwrap_or("");
            
            print!("\r🔍 {}... ", path);
            io::stdout().flush().unwrap();
            
            let content = fs::read_to_string(path).unwrap_or_else(|_| "Not found".to_string());
            
            println!("✅");
            
            let final_res = client.post(url)
                .json(&json!({
                    "model": model,
                    "messages": [
                        {"role": "user", "content": prompt},
                        {"role": "assistant", "content": null, "tool_calls": tool_calls},
                        {"role": "tool", "content": content}
                    ],
                    "stream": false
                }))
                .send().await?;
            
            json = final_res.json().await?;
        }
    }
    
    if let Some(msg) = json["message"].as_object() {
        if let Some(content) = msg.get("content") {
            let answer = content.as_str().unwrap_or("");
            let lines: Vec<&str> = answer.lines().take(5).collect();
            
            let spinner = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            for _ in 0..12 {
                for &c in &spinner {
                    print!("\r{}", c);
                    io::stdout().flush().unwrap();
                    thread::sleep(Duration::from_millis(40));
                }
            }
            print!("\r\x1B[K");
            io::stdout().flush().unwrap();
            
            chirp();
            return Ok(lines.join("\n"));
        }
    }
    
    Ok("".to_string())
}
