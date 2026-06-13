use std::io::Write;
use std::sync::OnceLock;
use tokio::sync::watch;
use tokio::time::{sleep, Duration};

const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub struct SpinnerHandle {
    stop_tx: watch::Sender<bool>,
}

pub fn start_spinner() -> SpinnerHandle {
    let (stop_tx, mut stop_rx) = watch::channel(false);
    let global_tx = stop_tx.clone();
    let _ = STOP_SENDER.set(global_tx);

    tokio::spawn(async move {
        let mut i = 0usize;
        loop {
            tokio::select! {
                _ = stop_rx.changed() => break,
                _ = sleep(Duration::from_millis(80)) => {
                    eprint!("\r{} ", FRAMES[i % FRAMES.len()]);
                    let _ = std::io::stderr().flush();
                    i += 1;
                }
            }
        }
    });

    SpinnerHandle { stop_tx }
}

pub static STOP_SENDER: OnceLock<watch::Sender<bool>> = OnceLock::new();

pub fn stop_global() {
    if let Some(tx) = STOP_SENDER.get() {
        let _ = tx.send(true);
    }
    eprint!("\r\x1B[2K\r");
    let _ = std::io::stderr().flush();
}

impl SpinnerHandle {
    pub fn stop(&self) {
        let _ = self.stop_tx.send(true);
        eprint!("\r\x1B[2K\r");
        let _ = std::io::stderr().flush();
    }
}
