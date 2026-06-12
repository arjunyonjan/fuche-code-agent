use colored::*;

pub fn header(model: &str) {
    println!("\n{}", "╔════════════════════════════════════════════╗".cyan());
    println!("{}", "║  ███████╗██╗   ██╗ ██████╗██╗  ██╗███████╗    ║".cyan());
    println!("{}", "║  ██╔════╝██║   ██║██╔════╝██║  ██║██╔════╝    ║".cyan());
    println!("{}", "║  █████╗  ██║   ██║██║     ███████║█████╗      ║".cyan());
    println!("{}", "║  ██╔══╝  ██║   ██║██║     ██╔══██║██╔══╝      ║".cyan());
    println!("{}", "║  ██║     ╚██████╔╝╚██████╗██║  ██║███████╗    ║".cyan());
    println!("{}", "║  ╚═╝      ╚═════╝  ╚═════╝╚═╝  ╚═╝╚══════╝    ║".cyan());
    println!("{}", "╠════════════════════════════════════════════╣".cyan());
    println!("║  {} {}  ║", "🔥 FUCHE CODE".bright_red().bold(), "v1.0".yellow());
    println!("║  {} {}  ║", "🤖 Model:".green(), model.cyan());
    println!("{}", "╚════════════════════════════════════════════╝".cyan());
    println!();
}

pub fn user_prompt() {
    print!("{}", "┌─[YOU]\n│ ".bright_blue());
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
}

pub fn bot_prefix() {
    println!("{}", "├─[FUCHE]".bright_magenta());
    print!("{}", "│ ".bright_magenta());
}
