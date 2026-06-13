use std::fs::{self, OpenOptions};
use std::io::{Write, BufRead, BufReader};
use serde_json::{json, Value};
use std::path::PathBuf;

fn get_history_path() -> PathBuf {
    dirs::home_dir().expect("HOME not set").join(".fuchecode_history.json")
}

pub fn save(user_msg: &str, assistant_msg: &str) {
    let path = get_history_path();
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let entry = json!({
            "role": "user",
            "content": user_msg,
            "timestamp": chrono::Utc::now().timestamp()
        });
        let _ = writeln!(file, "{entry}");

        let entry = json!({
            "role": "assistant",
            "content": assistant_msg,
            "timestamp": chrono::Utc::now().timestamp()
        });
        let _ = writeln!(file, "{entry}");
    }
}

pub fn load_last(n: usize) -> Vec<Value> {
    let path = get_history_path();
    if !path.exists() { return vec![]; }
    
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return vec![],
    };
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
    let last_n = lines.iter().rev().take(n).rev();
    
    last_n.filter_map(|line| serde_json::from_str(line).ok()).collect()
}

pub fn clear() {
    let path = get_history_path();
    let _ = fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clear_missing_file_no_panic() {
        clear();
    }

    #[test]
    fn test_history_path_has_correct_name() {
        let path = get_history_path();
        assert!(path.to_string_lossy().ends_with(".fuchecode_history.json"));
    }

    #[test]
    fn test_jsonl_roundtrip() {
        let path = PathBuf::from("/tmp/fuchecode_test_jsonl.json");
        {
            let mut file = OpenOptions::new().create(true).append(true).open(&path).unwrap();
            let entry = json!({"role": "user", "content": "hi", "timestamp": 0});
            writeln!(file, "{entry}").unwrap();
            let entry = json!({"role": "assistant", "content": "hey", "timestamp": 1});
            writeln!(file, "{entry}").unwrap();
        }

        let file = fs::File::open(&path).unwrap();
        let reader = BufReader::new(file);
        let lines: Vec<Value> = reader.lines()
            .map_while(Result::ok)
            .filter_map(|line| serde_json::from_str(&line).ok())
            .collect();

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["role"], "user");
        assert_eq!(lines[0]["content"], "hi");
        assert_eq!(lines[1]["role"], "assistant");
        assert_eq!(lines[1]["content"], "hey");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_corrupted_line_skipped() {
        let path = PathBuf::from("/tmp/fuchecode_test_corrupt.json");
        {
            let mut file = fs::File::create(&path).unwrap();
            writeln!(file, r#"{{"role": "user", "content": "good", "timestamp": 1}}"#).unwrap();
            writeln!(file, "not valid json").unwrap();
            writeln!(file, r#"{{"role": "assistant", "content": "ok", "timestamp": 2}}"#).unwrap();
        }

        let file = fs::File::open(&path).unwrap();
        let reader = BufReader::new(file);
        let lines: Vec<Value> = reader.lines()
            .map_while(Result::ok)
            .filter_map(|line| serde_json::from_str(&line).ok())
            .collect();

        assert_eq!(lines.len(), 2);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_empty_file() {
        let path = PathBuf::from("/tmp/fuchecode_test_empty.json");
        fs::write(&path, "").unwrap();

        let file = fs::File::open(&path).unwrap();
        let reader = BufReader::new(file);
        let lines: Vec<Value> = reader.lines()
            .map_while(Result::ok)
            .filter_map(|line| serde_json::from_str(&line).ok())
            .collect();

        assert!(lines.is_empty());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_jsonl_last_n_filter() {
        let path = PathBuf::from("/tmp/fuchecode_test_lastn.json");
        {
            let mut file = fs::File::create(&path).unwrap();
            for i in 0..5 {
                let entry = json!({"role": "user", "content": i.to_string(), "timestamp": i});
                writeln!(file, "{entry}").unwrap();
            }
        }

        let file = fs::File::open(&path).unwrap();
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
        let last_n: Vec<Value> = lines.iter()
            .rev().take(3).rev()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();

        assert_eq!(last_n.len(), 3);
        assert_eq!(last_n[0]["content"], "2");
        assert_eq!(last_n[2]["content"], "4");
        let _ = fs::remove_file(&path);
    }
}
