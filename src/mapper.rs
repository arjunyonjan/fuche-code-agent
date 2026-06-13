#[derive(Debug, PartialEq)]
pub enum Action {
    Execute(String),
    PassThrough,
}

fn esc(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn sed_repl(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('&', "\\&")
        .replace('|', "\\|")
        .replace('\'', "'\\''")
}

pub fn map(input: &str) -> Action {
    let t = input.trim();
    let lower = t.to_lowercase();
    let (first, rest) = t.split_once(' ').unwrap_or((t, ""));
    let rest = rest.trim();

    match first.to_ascii_lowercase().as_str() {
        "open" if !rest.is_empty() => {
            Action::Execute(format!("xdg-open {}", esc(rest)))
        }

        "cat" | "read" if !rest.is_empty() => {
            Action::Execute(format!("cat {}", esc(rest)))
        }

        "ls" | "list" => {
            let path = if rest.is_empty() { "." } else { rest };
            Action::Execute(format!("ls -la {}", esc(path)))
        }

        "run" if !rest.is_empty() => {
            Action::Execute(rest.to_string())
        }

        "live-server" | "server"
            if lower == "live-server" || lower.starts_with("live-server ") =>
        {
            Action::Execute("npx live-server".into())
        }

        _ if lower == "start server" => {
            Action::Execute("npx live-server".into())
        }

        "add" => {
            if let Some(pos) = rest.rfind(" to ") {
                let text = rest[..pos].trim();
                let file = rest[pos + 4..].trim();
                if !text.is_empty() && !file.is_empty() {
                    if file.ends_with(".html") || file.ends_with(".htm") {
                        return Action::Execute(format!(
                            "sed -i 's|</body>|{}\\n</body>|' {}",
                            sed_repl(text),
                            esc(file)
                        ));
                    }
                    return Action::Execute(format!(
                        "printf '%s\\n' {} >> {}",
                        esc(text),
                        esc(file)
                    ));
                }
            }
            Action::PassThrough
        }

        "search" => {
            if rest.is_empty() {
                return Action::PassThrough;
            }
            let (pat, path) = if let Some(pos) = rest.rfind(" in ") {
                (&rest[..pos], rest[pos + 4..].trim())
            } else {
                (rest, ".")
            };
            Action::Execute(format!("grep -rn {} {}", esc(pat.trim()), esc(path)))
        }

        _ => Action::PassThrough,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_file() {
        assert_eq!(map("open foo.txt"), Action::Execute("xdg-open 'foo.txt'".into()));
    }

    #[test]
    fn test_open_empty_returns_passthrough() {
        assert_eq!(map("open "), Action::PassThrough);
    }

    #[test]
    fn test_open_with_spaces() {
        assert_eq!(
            map("open my file.txt"),
            Action::Execute("xdg-open 'my file.txt'".into())
        );
    }

    #[test]
    fn test_cat_file() {
        assert_eq!(map("cat foo.rs"), Action::Execute("cat 'foo.rs'".into()));
    }

    #[test]
    fn test_read_alias() {
        assert_eq!(map("read foo.rs"), Action::Execute("cat 'foo.rs'".into()));
    }

    #[test]
    fn test_cat_empty_returns_passthrough() {
        assert_eq!(map("cat "), Action::PassThrough);
    }

    #[test]
    fn test_ls_default_path() {
        assert_eq!(map("ls"), Action::Execute("ls -la '.'".into()));
    }

    #[test]
    fn test_ls_with_path() {
        assert_eq!(map("ls src"), Action::Execute("ls -la 'src'".into()));
    }

    #[test]
    fn test_list_alias_default_path() {
        assert_eq!(map("list"), Action::Execute("ls -la '.'".into()));
    }

    #[test]
    fn test_list_with_path() {
        assert_eq!(map("list src"), Action::Execute("ls -la 'src'".into()));
    }

    #[test]
    fn test_run_command() {
        assert_eq!(map("run echo hi"), Action::Execute("echo hi".into()));
    }

    #[test]
    fn test_run_empty_returns_passthrough() {
        assert_eq!(map("run "), Action::PassThrough);
    }

    #[test]
    fn test_live_server() {
        assert_eq!(map("live-server"), Action::Execute("npx live-server".into()));
    }

    #[test]
    fn test_live_server_with_args() {
        assert_eq!(map("live-server --port=3000"), Action::Execute("npx live-server".into()));
    }

    #[test]
    fn test_start_server_alias() {
        assert_eq!(map("start server"), Action::Execute("npx live-server".into()));
    }

    #[test]
    fn test_add_to_html_file() {
        let result = map("add <p>hi</p> to index.html");
        assert!(matches!(result, Action::Execute(ref s) if s.contains("sed -i")));
    }

    #[test]
    fn test_add_to_txt_file() {
        let result = map("add hello world to notes.txt");
        assert!(matches!(result, Action::Execute(ref s) if s.contains("printf")));
    }

    #[test]
    fn test_add_without_to_returns_passthrough() {
        assert_eq!(map("add something"), Action::PassThrough);
    }

    #[test]
    fn test_add_empty_text_returns_passthrough() {
        assert_eq!(map("add  to file.txt"), Action::PassThrough);
    }

    #[test]
    fn test_search_with_in() {
        let result = map("search fn main in src/");
        assert_eq!(result, Action::Execute("grep -rn 'fn main' 'src/'".into()));
    }

    #[test]
    fn test_search_without_in_defaults_to_dot() {
        let result = map("search foobar");
        assert_eq!(result, Action::Execute("grep -rn 'foobar' '.'".into()));
    }

    #[test]
    fn test_search_empty_returns_passthrough() {
        assert_eq!(map("search "), Action::PassThrough);
    }

    #[test]
    fn test_empty_input_returns_passthrough() {
        assert_eq!(map(""), Action::PassThrough);
        assert_eq!(map("   "), Action::PassThrough);
    }

    #[test]
    fn test_unknown_command_returns_passthrough() {
        assert_eq!(map("foobar xyz"), Action::PassThrough);
    }

    #[test]
    fn test_case_insensitive_ls() {
        assert_eq!(map("LS src"), Action::Execute("ls -la 'src'".into()));
    }

    #[test]
    fn test_case_insensitive_cat() {
        assert_eq!(map("CAT foo.txt"), Action::Execute("cat 'foo.txt'".into()));
    }

    #[test]
    fn test_case_insensitive_live_server() {
        assert_eq!(map("Live-Server"), Action::Execute("npx live-server".into()));
    }

    #[test]
    fn test_open_with_apostrophe() {
        assert_eq!(
            map("open it's.txt"),
            Action::Execute("xdg-open 'it'\\''s.txt'".into())
        );
    }

    #[test]
    fn test_search_with_apostrophe() {
        assert_eq!(
            map("search it's in src/"),
            Action::Execute("grep -rn 'it'\\''s' 'src/'".into())
        );
    }

    #[test]
    fn test_add_with_apostrophe_in_text() {
        let result = map("add Arjun's code to notes.txt");
        assert!(matches!(result, Action::Execute(ref s) if s.contains("printf")));
        assert!(matches!(result, Action::Execute(ref s) if s.contains("Arjun")));
    }

    #[test]
    fn test_run_keeps_arguments() {
        assert_eq!(
            map("run npx live-server --port 8080"),
            Action::Execute("npx live-server --port 8080".into())
        );
    }

    #[test]
    fn test_single_word_passthrough() {
        assert_eq!(map("hello"), Action::PassThrough);
    }
}
