use reqwest;
use serde_json::{json, Value};
use crate::tools;
use crate::history;
use std::io::{self, Write};
use std::time::Instant;
use futures_util::StreamExt;

pub async fn chat(url: &str, model: &str, prompt: &str) -> Result<String, reqwest::Error> {
    let start = Instant::now();
    let client = reqwest::Client::new();
    
    let mut messages = history::load_last(10);
    messages.push(json!({"role": "user", "content": prompt}));
    
    let res = client.post(url)
        .json(&json!({
            "model": model,
            "messages": messages,
            "stream": true,
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": "read_file",
                        "description": "Read file",
                        "parameters": {
                            "type": "object",
                            "properties": { "path": {"type": "string"} },
                            "required": ["path"]
                        }
                    }
                },
                {
                    "type": "function",
                    "function": {
                        "name": "list_dir",
                        "description": "List directory",
                        "parameters": {
                            "type": "object",
                            "properties": { "path": {"type": "string"} }
                        }
                    }
                },
                {
                    "type": "function",
                    "function": {
                        "name": "search_content",
                        "description": "Search for text pattern in files",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "pattern": {"type": "string"},
                                "path": {"type": "string"}
                            },
                            "required": ["pattern"]
                        }
                    }
                },
                {
                    "type": "function",
                    "function": {
                        "name": "clear_screen",
                        "description": "Clear terminal",
                        "parameters": {
                            "type": "object",
                            "properties": {}
                        }
                    }
                }
            ]
        }))
        .send().await?;
    
    let mut stream = res.bytes_stream();
    let mut full_response = String::new();
    let mut buffer = Vec::new();
    
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buffer.extend_from_slice(&chunk);
        
        while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = buffer.drain(0..=pos).collect();
            let line = String::from_utf8_lossy(&line).trim().to_string();
            if line.is_empty() { continue; }
            
            if let Ok(json) = serde_json::from_str::<Value>(&line) {
                if json["done"].as_bool().unwrap_or(false) { break; }
                
                if let Some(content) = json["message"]["content"].as_str() {
                    print!("{}", content);
                    io::stdout().flush().unwrap();
                    full_response.push_str(content);
                }
                
                // Handle tool calls in stream
                if let Some(tool_calls) = json["message"]["tool_calls"].as_array() {
                    if !tool_calls.is_empty() {
                        for tool in tool_calls {
                            let name = tool["function"]["name"].as_str().unwrap_or("");
                            let args = &tool["function"]["arguments"];
                            
                            let result = match name {
                                "read_file" => tools::read_file(args["path"].as_str().unwrap_or("")),
                                "list_dir" => tools::list_dir(args["path"].as_str().unwrap_or("")),
                                "search_content" => tools::search_content(
                                    args["pattern"].as_str().unwrap_or(""),
                                    args["path"].as_str().unwrap_or("")
                                ),
                                "clear_screen" => {
                                    print!("\x1B[2J\x1B[1;1H");
                                    "Screen cleared".to_string()
                                }
                                _ => "Unknown".to_string(),
                            };
                            
                            eprintln!("\n✓ {} executed", name);
                            
                            // Send tool result back
                            let tool_res = client.post(url)
                                .json(&json!({
                                    "model": model,
                                    "messages": [
                                        {"role": "user", "content": prompt},
                                        {"role": "assistant", "content": null, "tool_calls": [tool]},
                                        {"role": "tool", "content": result}
                                    ],
                                    "stream": true
                                }))
                                .send().await?;
                            
                            let mut tool_stream = tool_res.bytes_stream();
                            let mut tool_buf = Vec::new();
                            while let Some(tool_chunk) = tool_stream.next().await {
                                let tool_chunk = tool_chunk?;
                                tool_buf.extend_from_slice(&tool_chunk);
                                while let Some(tpos) = tool_buf.iter().position(|&b| b == b'\n') {
                                    let tline: Vec<u8> = tool_buf.drain(0..=tpos).collect();
                                    let tline = String::from_utf8_lossy(&tline).trim().to_string();
                                    if tline.is_empty() { continue; }
                                    if let Ok(tjson) = serde_json::from_str::<Value>(&tline) {
                                        if tjson["done"].as_bool().unwrap_or(false) { break; }
                                        if let Some(content) = tjson["message"]["content"].as_str() {
                                            print!("{}", content);
                                            io::stdout().flush().unwrap();
                                            full_response.push_str(content);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    let elapsed = start.elapsed().as_millis();
    eprintln!("\n💭 {}ms", elapsed);
    
    history::save(prompt, &full_response);
    Ok(full_response)
}
