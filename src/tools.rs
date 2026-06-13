#![allow(dead_code)]

use std::fs;
use std::io::{self, Write};
use std::sync::OnceLock;
use crate::mode::ToolPermission;

fn is_wsl() -> bool {
    static WSL: OnceLock<bool> = OnceLock::new();
    *WSL.get_or_init(|| {
        std::env::var("WSL_DISTRO_NAME").is_ok()
            || fs::read_to_string("/proc/version")
                .map(|s| s.to_lowercase().contains("microsoft"))
                .unwrap_or(false)
    })
}

fn adapt_command(cmd: &str) -> String {
    let trimmed = cmd.trim();
    if trimmed.starts_with("xdg-open ") || trimmed.starts_with("open ") {
        let raw = trimmed
            .strip_prefix("xdg-open ")
            .or_else(|| trimmed.strip_prefix("open "))
            .unwrap_or("")
            .trim();
        if !raw.is_empty() {
            let bare = if raw.starts_with('\'') && raw.ends_with('\'') && raw.len() > 1 {
                &raw[1..raw.len()-1]
            } else {
                raw
            };
            if is_wsl() {
                return format!("cmd.exe /c start \"\" \"$(wslpath -w '{}')\"", bare.replace('\'', "'\\''"));
            }
            if cfg!(target_os = "windows") {
                return format!("start \"\" \"{}\"", bare);
            }
        }
    }
    cmd.to_string()
}

pub fn read_file(path: &str, perms: &ToolPermission) -> String {
    if !perms.read {
        return "⛔ Read denied in current mode".to_string();
    }
    print!("\r📖 {}... ", path);
    let _ = io::stdout().flush();
    fs::read_to_string(path).unwrap_or_else(|_| "Not found".to_string())
}

pub fn list_dir(path: &str, perms: &ToolPermission) -> String {
    if !perms.read {
        return "⛔ Read denied in current mode".to_string();
    }
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

pub fn search_content(pattern: &str, path: &str, perms: &ToolPermission) -> String {
    if !perms.read {
        return "⛔ Read denied in current mode".to_string();
    }
    let path = if path.is_empty() { "." } else { path };
    print!("\r🔍 '{}' in {}... ", pattern, path);
    let _ = io::stdout().flush();
    
    let mut results = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file()
                && let Ok(content) = fs::read_to_string(&p)
                && content.contains(pattern)
                && let Some(name) = p.file_name()
            {
                results.push(name.to_string_lossy().to_string());
            }
        }
    }
    if results.is_empty() { "No matches".to_string() } 
    else { format!("Found in: {}", results.join(", ")) }
}

pub fn write_file(path: &str, content: &str, perms: &ToolPermission) -> String {
    if !perms.write {
        return "⛔ Write denied in current mode".to_string();
    }
    match std::fs::write(path, content) {
        Ok(_) => format!("✅ Written to: {}", path),
        Err(e) => format!("❌ Error: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn build_perms() -> ToolPermission {
        ToolPermission { read: true, write: true, execute: true, network: true }
    }

    fn plan_perms() -> ToolPermission {
        ToolPermission { read: true, write: false, execute: false, network: false }
    }

    #[test]
    fn test_read_file_not_found() {
        let result = read_file("/tmp/nonexistent_file_12345.txt", &build_perms());
        assert_eq!(result, "Not found");
    }

    #[test]
    fn test_read_file_empty() {
        let path = "/tmp/fuchecode_test_empty.txt";
        fs::write(path, "").unwrap();
        let result = read_file(path, &build_perms());
        assert_eq!(result, "");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_read_file_with_content() {
        let path = "/tmp/fuchecode_test_content.txt";
        fs::write(path, "hello world").unwrap();
        let result = read_file(path, &build_perms());
        assert_eq!(result, "hello world");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_list_dir_empty_path() {
        let result = list_dir("", &build_perms());
        assert!(!result.is_empty());
    }

    #[test]
    fn test_list_dir_nonexistent() {
        let result = list_dir("/tmp/fuchecode_nonexistent_dir_xyz", &build_perms());
        assert!(result.starts_with("Error:"));
    }

    #[test]
    fn test_list_dir_with_files() {
        let dir = "/tmp/fuchecode_test_listdir";
        let _ = fs::create_dir_all(dir);
        fs::write(format!("{dir}/a.txt"), "a").unwrap();
        let result = list_dir(dir, &build_perms());
        assert!(result.contains("a.txt"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_search_content_no_match() {
        let result = search_content("zzz_xyz_nonexistent", "/tmp", &build_perms());
        assert_eq!(result, "No matches");
    }

    #[test]
    fn test_search_content_finds_match() {
        let dir = "/tmp/fuchecode_test_search";
        let _ = fs::create_dir_all(dir);
        fs::write(format!("{dir}/findme.txt"), "secret_keyword_42").unwrap();
        let result = search_content("secret_keyword_42", dir, &build_perms());
        assert!(result.contains("findme.txt"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_search_content_empty_pattern() {
        let dir = "/tmp/fuchecode_test_search_empty";
        let _ = fs::create_dir_all(dir);
        fs::write(format!("{dir}/any.txt"), "data").unwrap();
        let result = search_content("", dir, &build_perms());
        assert!(result.contains("any.txt"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_write_file_success() {
        let path = "/tmp/fuchecode_test_write.txt";
        let result = write_file(path, "test content", &build_perms());
        assert!(result.contains("✅ Written to"));

        let content = fs::read_to_string(path).unwrap();
        assert_eq!(content, "test content");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_write_file_invalid_path() {
        let result = write_file("/dev/null/fuchecode_test", "data", &build_perms());
        assert!(result.starts_with("❌ Error:"));
    }

    #[test]
    fn test_list_dir_special_chars_in_name() {
        let dir = "/tmp/fuchecode_test_special";
        let _ = fs::create_dir_all(dir);
        let file_path = format!("{dir}/file with spaces!@#.txt");
        fs::write(&file_path, "data").unwrap();
        let result = list_dir(dir, &build_perms());
        assert!(result.contains("file with spaces!@#.txt"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_write_denied_in_plan_mode() {
        let result = write_file("/tmp/fuchecode_denied.txt", "data", &plan_perms());
        assert!(result.contains("⛔"));
        assert!(result.contains("denied"));
    }

    #[test]
    fn test_read_allowed_in_plan_mode() {
        let path = "/tmp/fuchecode_plan_read.txt";
        fs::write(path, "visible").unwrap();
        let result = read_file(path, &plan_perms());
        assert_eq!(result, "visible");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_edit_file_replaces_text() {
        let path = "/tmp/fuchecode_edit_test.txt";
        fs::write(path, "hello world foo").unwrap();
        let result = edit_file(path, "foo", "bar", &build_perms());
        assert!(result.contains("✅ Edited"));
        let content = fs::read_to_string(path).unwrap();
        assert_eq!(content, "hello world bar");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_edit_file_pattern_not_found() {
        let path = "/tmp/fuchecode_edit_notfound.txt";
        fs::write(path, "hello").unwrap();
        let result = edit_file(path, "nonexistent", "bar", &build_perms());
        assert!(result.contains("❌ Pattern not found"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_edit_file_denied_in_plan() {
        let result = edit_file("/tmp/any.txt", "a", "b", &plan_perms());
        assert!(result.contains("⛔"));
    }

    #[test]
    fn test_delete_file_success() {
        let path = "/tmp/fuchecode_delete_test.txt";
        fs::write(path, "data").unwrap();
        let result = delete_file(path, &build_perms());
        assert!(result.contains("✅ Deleted"));
        assert!(!std::path::Path::new(path).exists());
    }

    #[test]
    fn test_delete_file_not_found() {
        let result = delete_file("/tmp/fuchecode_nonexistent_xyz.txt", &build_perms());
        assert!(result.contains("❌"));
    }

    #[test]
    fn test_delete_file_denied_in_plan() {
        let result = delete_file("/tmp/any.txt", &plan_perms());
        assert!(result.contains("⛔"));
    }
}

pub fn edit_file(path: &str, old: &str, new: &str, perms: &ToolPermission) -> String {
    if !perms.write {
        return "⛔ Write denied in current mode".to_string();
    }
    match fs::read_to_string(path) {
        Ok(content) => {
            if !content.contains(old) {
                return format!("❌ Pattern not found in: {}", path);
            }
            let replaced = content.replace(old, new);
            match fs::write(path, &replaced) {
                Ok(_) => format!("✅ Edited: {}", path),
                Err(e) => format!("❌ Error writing: {}", e),
            }
        }
        Err(e) => format!("❌ Error reading: {}", e),
    }
}

pub fn delete_file(path: &str, perms: &ToolPermission) -> String {
    if !perms.write {
        return "⛔ Write denied in current mode".to_string();
    }
    match fs::remove_file(path) {
        Ok(_) => format!("✅ Deleted: {}", path),
        Err(e) => format!("❌ Error: {}", e),
    }
}

fn is_dangerous(command: &str) -> Option<&'static str> {
    let lower = command.to_lowercase();
    let patterns: &[(&str, &str)] = &[
        ("sudo ", "Use of sudo is blocked — run dangerous commands in your terminal directly"),
        ("npm install -g", "Global npm install is blocked"),
        ("npm i -g", "Global npm install is blocked"),
        ("chmod +x", "chmod +x is blocked"),
        ("chmod 777", "chmod 777 is blocked"),
        ("chmod 4755", "SUID bit changes are blocked"),
        ("| bash", "Piping to shell is blocked"),
        ("| sh", "Piping to shell is blocked"),
        ("| zsh", "Piping to shell is blocked"),
        ("| fish", "Piping to shell is blocked"),
        ("mkfs", "Filesystem creation is blocked"),
        ("dd if=", "Raw disk writes are blocked"),
        ("> /dev/sda", "Disk device writes are blocked"),
        ("> /dev/nvme", "Disk device writes are blocked"),
        (":(){ :|:& };:", "Fork bombs are blocked"),
        ("wget -O -", "Piping web content to shell is blocked"),
    ];
    for (pat, msg) in patterns {
        if lower.contains(pat) {
            return Some(msg);
        }
    }
    None
}

pub fn run_command(command: &str, perms: &ToolPermission) -> String {
    if !perms.execute {
        return "⛔ Execute denied in current mode".to_string();
    }
    if command.trim().is_empty() {
        return "⛔ Error: empty command. Call run_command with a real command.".to_string();
    }
    let command = adapt_command(command);
    if let Some(reason) = is_dangerous(&command) {
        return format!("⛔ Blocked: {}", reason);
    }
    crate::spinner::stop_global();
    let display = if command.len() > 120 {
        format!("{}...", &command[..117])
    } else {
        command.clone()
    };
    print!("\x1B[2K\r⚡ {}\n", display);
    let _ = std::io::stdout().flush();
    match std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
    {
        Ok(output) => {
            let out = String::from_utf8_lossy(&output.stdout);
            let err = String::from_utf8_lossy(&output.stderr);
            if output.status.success() {
                let trimmed = out.trim().to_string();
                if trimmed.is_empty() { "✅ Done.".to_string() } else { trimmed }
            } else {
                let code = output.status.code().unwrap_or(-1);
                let msg = err.trim();
                if msg.is_empty() { format!("Exit {}", code) } else { format!("Exit {}: {}", code, msg) }
            }
        }
        Err(e) => format!("❌ Error: {}", e),
    }
}
