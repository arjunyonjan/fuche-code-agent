use colored::*;
use std::io::Write;
use crate::mode::Mode;

pub fn header(model: &str) {
    header_with_mode(model, &Mode::Build);
}

pub fn header_with_mode(model: &str, mode: &Mode) {
    let mode_tag = format!("{} {}", match mode { Mode::Build => "🔵", Mode::Plan => "🟠" }, mode.name());
    print!("\r{}\n", "╔════════════════════════════════════════════╗".cyan());
    print!("\r{}\n", "║  ███████╗██╗   ██╗ ██████╗██╗  ██╗███████╗    ║".cyan());
    print!("\r{}\n", "║  ██╔════╝██║   ██║██╔════╝██║  ██║██╔════╝    ║".cyan());
    print!("\r{}\n", "║  █████╗  ██║   ██║██║     ███████║█████╗      ║".cyan());
    print!("\r{}\n", "║  ██╔══╝  ██║   ██║██║     ██╔══██║██╔══╝      ║".cyan());
    print!("\r{}\n", "║  ██║     ╚██████╔╝╚██████╗██║  ██║███████╗    ║".cyan());
    print!("\r{}\n", "║  ╚═╝      ╚═════╝  ╚═════╝╚═╝  ╚═╝╚══════╝    ║".cyan());
    print!("\r{}\n", "╠════════════════════════════════════════════╣".cyan());
    print!("\r║  {} {}  {:>12}  ║\n", "🔥 FUCHECODE".bright_red().bold(), "v1.0".yellow(), mode_tag.cyan());
    print!("\r║  {} {}  ║\n", "🤖 Model:".green(), model.cyan());
    print!("\r{}\n", "╚════════════════════════════════════════════╝".cyan());
    print!("\r\n");
    std::io::stdout().flush().ok();
}

pub fn user_prompt_with_mode(mode: &Mode) {
    let prefix = match mode {
        Mode::Build => "┌─[BUILD]".bright_blue(),
        Mode::Plan => "┌─[PLAN]".bright_yellow(),
    };
    print!("\r{}\n\r│ ", prefix);
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
}

pub fn bot_prefix_with_mode(mode: &Mode) {
    let prefix = match mode {
        Mode::Build => "├─[BUILD]".bright_blue(),
        Mode::Plan => "├─[PLAN]".bright_yellow(),
    };
    print!("\r{}\n", prefix);
    print!("\r│ ");
}
