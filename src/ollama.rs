use reqwest;
use serde_json::{json, Value};
use crate::tools;
use crate::spinner;

pub async fn chat(url: &str, model: &str, prompt: &str) -> Result<String, reqwest::Error> {
    let client = reqwest::Client::new();
    
    let res = client.post(url)
        .json(&json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "stream": false,
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
                }
            ]
        }))
        .send().await?;
    
    let mut json: Value = res.json().await?;
    
    if let Some(tool_calls) = json["message"]["tool_calls"].as_array() {
        if !tool_calls.is_empty() {
            let tool = &tool_calls[0];
            let result = match tool["function"]["name"].as_str().unwrap_or("") {
                "read_file" => tools::read_file(tool["function"]["arguments"]["path"].as_str().unwrap_or("")),
                "list_dir" => tools::list_dir(tool["function"]["arguments"]["path"].as_str().unwrap_or("")),
                _ => "Unknown".to_string(),
            };
            println!(" ✅");
            
            let final_res = client.post(url)
                .json(&json!({
                    "model": model,
                    "messages": [
                        {"role": "user", "content": prompt},
                        {"role": "assistant", "content": null, "tool_calls": tool_calls},
                        {"role": "tool", "content": result}
                    ],
                    "stream": false
                }))
                .send().await?;
            json = final_res.json().await?;
        }
    }
    
    let answer = json["message"]["content"].as_str().unwrap_or("");
    spinner::animate();
    spinner::chirp();
    Ok(answer.to_string())
}
