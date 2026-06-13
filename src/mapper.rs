pub enum Action {
    Execute(String),
    PassThrough,
}

fn esc(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

pub fn map(input: &str) -> Action {
    let t = input.trim();
    let lower = t.to_lowercase();

    if let Some(file) = t.strip_prefix("open ").map(|s| s.trim()).filter(|s| !s.is_empty()) {
        return Action::Execute(format!("xdg-open {}", esc(file)));
    }

    if lower == "live-server" || lower.starts_with("live-server ") || lower == "start server" {
        return Action::Execute("npx live-server".into());
    }

    if let Some(file) = t.strip_prefix("cat ").or(t.strip_prefix("read ")).map(|s| s.trim()).filter(|s| !s.is_empty()) {
        return Action::Execute(format!("cat {}", esc(file)));
    }

    if lower == "ls" || lower.starts_with("ls ") || lower == "list" || lower.starts_with("list ") {
        let path = t.splitn(2, ' ').nth(1).unwrap_or(".");
        return Action::Execute(format!("ls -la {}", esc(path)));
    }

    if let Some(rest) = t.strip_prefix("search ") {
        let (pat, path) = if let Some(pos) = rest.rfind(" in ") {
            (&rest[..pos], rest[pos + 4..].trim())
        } else {
            (rest, ".")
        };
        return Action::Execute(format!("grep -rn '{}' {}", esc(pat.trim()), esc(path)));
    }

    if let Some(cmd) = t.strip_prefix("run ").map(|s| s.trim()).filter(|s| !s.is_empty()) {
        return Action::Execute(cmd.to_string());
    }

    Action::PassThrough
}
