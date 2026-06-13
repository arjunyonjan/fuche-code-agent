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

pub fn show_guide(provider: &str, cr_online: bool) {
    let cr_status = if cr_online { "🟢 active".bright_green() } else { "🔴 offline".bright_red() };

    print!("\r{}\n", "┌─ Quick Guide ─────────────────────────────────┐".cyan());
    print!("\r│                                                 │\n");
    print!("\r│  {} Provider: {} ({})                    │\n", "📡".bright_green(), provider.cyan(), cr_status);
    print!("\r│  {} Switch:    /provider clawrouter / nvidia / ollama │\n", "🔄".bright_green());
    print!("\r│  {} Model:     /model auto                           │\n", "🤖".bright_green());

    if provider == "clawrouter" {
        print!("\r│  {} Run CR:    npx @blockrun/clawrouter@latest       │\n", "🚀".bright_green());
        if cr_online {
            print!("\r│  {}           🟢 Connected at :8402                   │\n", "   ".bright_green());
        } else {
            print!("\r│  {}           🔴 NOT RUNNING — start in another term │\n", "   ".bright_red());
        }
    } else {
        print!("\r│  {} (not ClawRouter — no health check)             │\n", "   ".dimmed());
    }
    print!("\r│                                                 │\n");
    print!("\r│  {} ─── Quick Start ───                             │\n", "📐".bright_yellow());
    print!("\r│  {} 1. /provider clawrouter                          │\n", "   ".cyan());
    print!("\r│  {} 2. /model auto                                   │\n", "   ".cyan());
    print!("\r│  {} 3. ask: create hello.html                        │\n", "   ".cyan());
    print!("\r│  {} 4. /mode    (toggle Build/Plan)                  │\n", "   ".cyan());
    print!("\r│  {} 5. /help    (all commands)                       │\n", "   ".cyan());
    print!("\r│                                                 │\n");
    print!("\r│  {} tab  — toggle mode    {} /  — palette    {} Ctrl+C — cancel │\n", "⌨️".dimmed(), "⌨️".dimmed(), "⌨️".dimmed());
    print!("\r{}\n", "└────────────────────────────────────────────────┘".cyan());
    print!("\r\n");
    std::io::stdout().flush().ok();
}

pub fn user_prompt_with_mode(mode: &Mode) {
    let prefix = match mode {
        Mode::Build => "┌─[BUILD]".bright_blue(),
        Mode::Plan => "┌─[PLAN]".bright_yellow(),
    };
    print!("\r{}\n\r│ ", prefix);
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

pub fn bot_prefix_with_mode(mode: &Mode) {
    let prefix = match mode {
        Mode::Build => "├─[BUILD]".bright_blue(),
        Mode::Plan => "├─[PLAN]".bright_yellow(),
    };
    print!("\r{}\n", prefix);
    print!("\r│ ");
}
