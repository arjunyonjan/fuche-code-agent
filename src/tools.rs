use std::fs;
use std::io::{self, Write};

pub fn read_file(path: &str) -> String {
    print!("\r📖 {}... ", path);
    let _ = io::stdout().flush();
    fs::read_to_string(path).unwrap_or_else(|_| "Not found".to_string())
}

pub fn list_dir(path: &str) -> String {
    let path = if path.is_empty() { "." } else { path };
    print!("\r📁 {}... ", path);
    let _ = io::stdout().flush();
    
    match fs::read_dir(path) {
        Ok(entries) => {
            let mut dirs = Vec::new();
            let mut files = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if entry.path().is_dir() {
                    dirs.push(format!("  📁 {}", name));
                } else {
                    files.push(format!("  📄 {}", name));
                }
            }
            dirs.sort(); files.sort();
            let mut output = vec!["📂 Current Directory:".to_string()];
            if !dirs.is_empty() { output.push("".to_string()); output.push("Directories:".to_string()); output.extend(dirs); }
            if !files.is_empty() { output.push("".to_string()); output.push("Files:".to_string()); output.extend(files); }
            output.join("\n")
        }
        Err(e) => format!("Error: {}", e),
    }
}

pub fn search_content(pattern: &str, path: &str) -> String {
    let path = if path.is_empty() { "." } else { path };
    print!("\r🔍 '{}' in {}... ", pattern, path);
    let _ = io::stdout().flush();
    
    let mut results = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Ok(content) = fs::read_to_string(&p) {
                    if content.contains(pattern) {
                        results.push(p.file_name().unwrap().to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    if results.is_empty() { "No matches".to_string() } 
    else { format!("Found in: {}", results.join(", ")) }
}
