use std::fs::{self, OpenOptions};
use std::io::{Write, BufRead, BufReader};
use serde_json::{json, Value};
use std::path::PathBuf;

fn get_history_path() -> PathBuf {
    dirs::home_dir().unwrap().join(".fuchecode_history.json")
}

pub fn save(user_msg: &str, assistant_msg: &str) {
    let path = get_history_path();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();
    
    let entry = json!({
        "role": "user",
        "content": user_msg,
        "timestamp": chrono::Utc::now().timestamp()
    });
    writeln!(file, "{}", entry.to_string()).unwrap();
    
    let entry = json!({
        "role": "assistant",
        "content": assistant_msg,
        "timestamp": chrono::Utc::now().timestamp()
    });
    writeln!(file, "{}", entry.to_string()).unwrap();
}

pub fn load_last(n: usize) -> Vec<Value> {
    let path = get_history_path();
    if !path.exists() { return vec![]; }
    
    let file = fs::File::open(path).unwrap();
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().filter_map(Result::ok).collect();
    let last_n = lines.iter().rev().take(n).rev();
    
    last_n.filter_map(|line| serde_json::from_str(line).ok()).collect()
}

pub fn clear() {
    let path = get_history_path();
    let _ = fs::remove_file(path);
}
