use std::io::{self, Write};
use std::thread;
use std::time::Duration;
use std::process::Command;

pub fn chirp() {
    let _ = Command::new("powershell.exe")
        .args(&["-c", "[System.Console]::Beep(1000, 100)"])
        .output();
}

pub fn animate() {
    let spinner = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    for _ in 0..4 {
        for &c in &spinner {
            print!("\r{}", c);
            let _ = io::stdout().flush();
            thread::sleep(Duration::from_millis(40));
        }
    }
    print!("\r\x1B[K");
    let _ = io::stdout().flush();
}
