use futures_util::StreamExt;
use reqwest;
use serde_json::{json, Value};
use std::io::Write;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use crate::mode::Mode;

fn make_client(timeout_secs: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

pub async fn check_cr_status() -> bool {
    let client = make_client(2);
    client.get("http://127.0.0.1:8402/v1/models")
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

pub async fn list_models(api_url: &str, timeout_secs: u64) -> Result<Vec<String>, reqwest::Error> {
    let base_url = if api_url.contains("8402") || api_url.contains("blockrun") {
        let pos = api_url.find("/v1/").unwrap_or(api_url.len());
        format!("{}/v1/models", &api_url[..pos])
    } else if api_url.contains("integrate.api.nvidia.com") {
        let pos = api_url.find("/v1/").unwrap_or(api_url.len());
        format!("{}/v1/models", &api_url[..pos])
    } else {
        let pos = api_url.find("/api/").unwrap_or(api_url.len());
        format!("{}/api/tags", &api_url[..pos])
    };

    let client = make_client(timeout_secs);
    let resp = client.get(&base_url).send().await?;
    let json: Value = resp.json().await?;

    let models = if base_url.contains("/api/tags") {
        json["models"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    } else {
        json["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    };

    Ok(models)
}

pub async fn single_turn(
    api_url: &str,
    model: &str,
    prompt: &str,
    _executed_cmd: &str,
    tool_result: &str,
    timeout_secs: u64,
) -> String {
    use std::time::Instant;
    let start = Instant::now();
    let client = make_client(timeout_secs);
    let is_ollama_api = api_url.contains("/api/chat");

    let system = format!("The command has been executed with this result:\n{}\nBriefly summarize what happened in 1 sentence. Do not show the result again.", tool_result);
    let messages = vec![
        json!({"role": "system", "content": system}),
        json!({"role": "user", "content": prompt}),
    ];

    let body = json!({
        "model": model,
        "messages": messages,
        "stream": false,
    });

    let mut req = client.post(api_url).json(&body);
    if api_url.contains("integrate.api.nvidia.com") {
        let key = std::env::var("NVIDIA_API_KEY").unwrap_or_default();
        if !key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
    }

    match req.send().await {
        Ok(resp) => {
            if !resp.status().is_success() {
                return format!("❌ API error: {}", resp.status());
            }
            let json: Value = resp.json().await.unwrap_or_default();
            let text = if is_ollama_api {
                json["message"]["content"].as_str().unwrap_or("")
            } else {
                json["choices"][0]["message"]["content"].as_str().unwrap_or("")
            };
            let elapsed = start.elapsed().as_millis();
            format!("💭 {}ms — {}\n{}", elapsed, model, text.trim())
        }
        Err(e) => format!("❌ Connection failed: {}", e),
    }
}

const SYSTEM_PROMPT: &str = "You MUST use the run_command tool for ANY terminal command, file operation, web request, or system task. NEVER generate fake command output yourself — always call the tool. After the tool returns, briefly summarize the result. For editing files use `sed -i` to edit in place. For opening files use `xdg-open <file>`. NEVER install packages or start servers unless the user explicitly asks. NEVER fake a result — always call the tool. When the user asks to update/create/edit/modify a file, you MUST call run_command to write the file. NEVER just show the new content in your response — write it to the file. Answer within 5 lines in simple layman terms. No jargon.";

fn tool_defs() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "run_command",
                "description": "Execute any shell command. Use this for creating/editing/deleting files, searching the web, git operations, opening browsers, installing packages, or any system task.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {"type": "string", "description": "Shell command to execute"}
                    },
                    "required": ["command"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "apply_patch",
                "description": "Apply surgical edits to a file. Prefer this over run_command with sed/cat for targeted changes. Each operation is applied in order on the same file. If any hunk fails the patch stops with an error message — the model can then retry with corrected hunks.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "file": {"type": "string", "description": "Path to the file to patch"},
                        "operations": {
                            "type": "array",
                            "description": "Ordered list of edit operations to apply",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "op": {
                                        "type": "string",
                                        "enum": ["search_replace", "delete", "insert_after", "insert_before"],
                                        "description": "search_replace: find 'old' and replace with 'new'. delete: remove 'old'. insert_after: place 'new' after 'old'. insert_before: place 'new' before 'old'."
                                    },
                                    "old": {"type": "string", "description": "Text to find (must exist in file)"},
                                    "new": {"type": "string", "description": "Replacement or insertion text"}
                                },
                                "required": ["op", "old"]
                            }
                        }
                    },
                    "required": ["file", "operations"]
                }
            }
        }
    ])
}

pub fn chat_stream(
    api_url: &str,
    model: &str,
    prompt: &str,
    timeout_secs: u64,
    mode: Mode,
    force_tool: bool,
) -> (mpsc::Receiver<String>, JoinHandle<()>) {
    let (tx, rx) = mpsc::channel::<String>(256);
    let url = api_url.to_string();
    let model = model.to_string();
    let prompt = prompt.to_string();

    let handle = tokio::spawn(async move {
        async fn fail(tx: &mpsc::Sender<String>, msg: &str) {
            let _ = tx.send(msg.to_string()).await;
            let _ = writeln!(std::io::stderr(), "{}", msg);
        }

        let perms = mode.permissions();
        let start = std::time::Instant::now();
        let client = make_client(timeout_secs);
        let is_nvidia = url.contains("integrate.api.nvidia.com");
        let is_clawrouter = url.contains("8402") || url.contains("blockrun");
        let is_ollama = !is_nvidia && !is_clawrouter;
        let mut actual_model = String::new();

        let mut messages: Vec<Value> = vec![
            json!({"role": "system", "content": SYSTEM_PROMPT}),
            json!({"role": "user", "content": prompt}),
        ];

        let tools = tool_defs();

        loop {
            let mut body = json!({
                "model": model,
                "messages": messages,
                "stream": true
            });

            if !is_ollama {
                body["tools"] = tools.clone();
                if force_tool {
                    body["tool_choice"] = json!("required");
                }
            }

            let mut req = client.post(&url).json(&body);

            if is_nvidia {
                let key = std::env::var("NVIDIA_API_KEY").unwrap_or_default();
                if key.is_empty() {
                    let _ = writeln!(std::io::stderr(), "❌ NVIDIA_API_KEY not set in environment");
                }
                req = req.header("Authorization", format!("Bearer {}", key));
            }

            let mut tool_calls: Vec<(usize, String, String, String)> = Vec::new();
            let mut text_content = String::new();

            let response_ok = match req.send().await {
                Ok(response) => {
                    let status = response.status();
                    if !status.is_success() {
                        let body_text = response.text().await.unwrap_or_default();
                        let err = format!("❌ API error {}: {}", status, body_text);
                        fail(&tx, &err).await;
                        false
                    } else {
                        let mut stream = response.bytes_stream();
                        let mut buf = String::new();
                        let mut got_content = false;

                        while let Some(chunk_result) = stream.next().await {
                            match chunk_result {
                                Ok(chunk) => {
                                    let chunk_str = String::from_utf8_lossy(&chunk);
                                    buf.push_str(&chunk_str);

                                    while let Some(nl) = buf.find('\n') {
                                        let line = buf[..nl].trim().to_string();
                                        buf = buf[nl + 1..].to_string();

                                        if line.is_empty() || line.starts_with(':') || line == "data: [DONE]" {
                                            continue;
                                        }

                                        if is_ollama {
                                            match serde_json::from_str::<Value>(&line) {
                                                Ok(v) => {
                                                    if let Some(text) = v["message"]["content"].as_str() {
                                                        got_content = true;
                                                        text_content.push_str(text);
                                                        let _ = tx.send(text.to_string()).await;
                                                    }
                                                }
                                                Err(e) => {
                                                    let _ = writeln!(std::io::stderr(), "⚠️ Parse warning: {} — line: {}", e, &line[..line.len().min(80)]);
                                                }
                                            }
                                        } else {
                                            let json_str = line.strip_prefix("data: ").unwrap_or(&line);
                                            match serde_json::from_str::<Value>(json_str) {
                                                Ok(v) => {
                                                    if actual_model.is_empty() {
                                                        if let Some(m) = v["model"].as_str() {
                                                            actual_model = m.to_string();
                                                        }
                                                    }
                                                    if let Some(err_msg) = v["error"].as_str() {
                                                        let err = format!("❌ API error: {}", err_msg);
                                                        fail(&tx, &err).await;
                                                        got_content = true;
                                                    } else if let Some(err_obj) = v["error"].as_object() {
                                                        let msg = err_obj.get("message").and_then(|m| m.as_str()).unwrap_or("unknown error");
                                                        let err = format!("❌ API error: {}", msg);
                                                        fail(&tx, &err).await;
                                                        got_content = true;
                                                    }
                                                    let delta = &v["choices"][0]["delta"];

                                                    if let Some(tcs) = delta["tool_calls"].as_array() {
                                                        for tc in tcs {
                                                            let idx = tc["index"].as_i64().unwrap_or(0) as usize;
                                                            let id = tc["id"].as_str().unwrap_or("").to_string();
                                                            let name = tc["function"]["name"].as_str().unwrap_or("").to_string();
                                                            let args = tc["function"]["arguments"].as_str().unwrap_or("").to_string();

                                                            if tool_calls.iter().any(|(i, _, _, _)| *i == idx) {
                                                                if let Some(entry) = tool_calls.iter_mut().find(|(i, _, _, _)| *i == idx) {
                                                                    if !id.is_empty() { entry.1 = id; }
                                                                    if !name.is_empty() { entry.2 = name; }
                                                                    entry.3.push_str(&args);
                                                                }
                                                            } else {
                                                                tool_calls.push((idx, id, name, args));
                                                            }
                                                        }
                                                    }

                                                    if let Some(text) = delta["content"].as_str() {
                                                        got_content = true;
                                                        text_content.push_str(text);
                                                        if !text.contains("⚠️ Wallet empty") {
                                                            let _ = tx.send(text.to_string()).await;
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    let _ = writeln!(std::io::stderr(), "⚠️ JSON parse error: {} — line: {}", e, json_str.chars().take(80).collect::<String>());
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    let err = format!("❌ Stream error: {}", e);
                                    fail(&tx, &err).await;
                                    break;
                                }
                            }
                        }

                        if !got_content && !is_ollama {
                            if let Ok(v) = serde_json::from_str::<Value>(&buf) {
                                let err_msg = v["error"].as_str()
                                    .or_else(|| v["error"].as_object()
                                        .and_then(|o| o.get("message"))
                                        .and_then(|m| m.as_str()))
                                    .unwrap_or("");
                                if !err_msg.is_empty() {
                                    let err = format!("❌ API error: {}", err_msg);
                                    fail(&tx, &err).await;
                                }
                            }
                        }

                        true
                    }
                }
                Err(e) => {
                    let err = format!("❌ Connection failed: {}", e);
                    fail(&tx, &err).await;
                    false
                }
            };

            if !response_ok {
                break;
            }

            let tool_calls: Vec<(String, String, String)> = tool_calls
                .into_iter()
                .filter(|(_, id, _, _)| !id.is_empty())
                .map(|(_, id, name, args)| (id, name, args))
                .collect();

            if tool_calls.is_empty() {
                break;
            }

            let mut tcs: Vec<Value> = Vec::new();
            for (id, name, args) in &tool_calls {
                tcs.push(json!({
                    "id": id,
                    "type": "function",
                    "function": { "name": name, "arguments": args }
                }));
            }
            let mut assistant_msg = json!({"role": "assistant", "content": text_content});
            assistant_msg["tool_calls"] = json!(tcs);
            messages.push(assistant_msg);

            for (id, name, args_json) in &tool_calls {
                let result = match name.as_str() {
                    "run_command" => {
                        let cmd = serde_json::from_str::<Value>(args_json)
                            .ok()
                            .and_then(|v| v["command"].as_str().map(|s| s.to_string()))
                            .unwrap_or_default();
                        crate::tools::run_command(&cmd, &perms)
                    }
                    "apply_patch" => {
                        let args: Value = serde_json::from_str(args_json).unwrap_or_default();
                        let file = args["file"].as_str().unwrap_or("").to_string();
                        let ops = args["operations"].as_array().cloned().unwrap_or_default();
                        crate::tools::apply_patch(&file, &ops, &perms)
                    }
                    _ => format!("❌ Unknown tool: {}", name),
                };
                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": id,
                    "content": result
                }));
            }
        }

        let elapsed = start.elapsed().as_millis();
        if actual_model.is_empty() {
            print!("\r\x1B[2K\r💭 {}ms", elapsed);
        } else {
            print!("\r\x1B[2K\r💭 {}ms — {}", elapsed, actual_model);
        }
        let _ = std::io::stdout().flush();
    });

    (rx, handle)
}
