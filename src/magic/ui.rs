use ratatui::layout::Rect;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use ratatui::widgets::canvas::{Painter, Shape};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use chrono::Local;

use crate::magic::audio::AudioState;

#[allow(dead_code)]
pub(super) struct WireframeGlobe {
    segments: Vec<[(f64, f64, f64); 2]>,
    rotation: f64,
}

impl WireframeGlobe {
    pub(super) fn new(_radius: f64) -> Self {
        Self { segments: vec![], rotation: 0.0 }
    }
}

impl Shape for WireframeGlobe {
    fn draw(&self, _painter: &mut Painter) {}
}

fn draw_spectrum(f: &mut Frame, area: Rect) {
    let chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let mut line = String::new();
    for i in 0..16 {
        let val = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_millis() as f32 / 1000.0 * 3.0 + i as f32 * 0.5).sin().abs();
        let idx = (val * 7.0) as usize;
        line.push(chars[idx.min(7)]);
    }
    let p = Paragraph::new(line).style(Style::default().fg(Color::Rgb(0, 230, 200)));
    f.render_widget(p, area);
}

pub fn draw(
    f: &mut Frame,
    _fps: u64,
    esc_pressed: bool,
    state: &Arc<Mutex<AudioState>>,
    track: &str,
    _last_key: &str,
    gpu_temp: f64,
    gpu_util: f64,
    gpu_mem_used: f64,
    gpu_mem_total: f64,
    ram_used: f64,
    ram_total: f64,
    _cpu_temp: Option<f64>,
    _cpu_freq: f64,
    _cpu_cores: usize,
    _core_pcts: &[f64],
    _sparkline: &VecDeque<u8>,
) {
    let fg = Style::default().fg(Color::Rgb(0, 230, 200));

    if esc_pressed {
        let confirm = Paragraph::new("Press Esc again to exit, or any other key to continue")
            .style(Style::default().fg(Color::Rgb(255, 200, 0)))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(confirm, f.area());
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)])
        .split(f.area());

    // Telemetry top bar
    let mut tel = String::new();
    if let Some(cpu_temp) = crate::magic::sys::read_cpu_temp() {
        tel.push_str(&format!("CPU {:.0}°  ", cpu_temp));
    }
    if gpu_temp > 0.0 {
        tel.push_str(&format!("GPU {:.0}° {:.0}%  ", gpu_temp, gpu_util));
    }
    if gpu_mem_total > 0.0 {
        tel.push_str(&format!("VRAM {:.1}/{:.1}G  ", gpu_mem_used, gpu_mem_total));
    }
    if ram_total > 0.0 {
        let pct = (ram_used / ram_total * 100.0) as u8;
        tel.push_str(&format!("RAM {:.1}/{:.1}G {}%", ram_used, ram_total, pct));
    }
    if !tel.is_empty() {
        f.render_widget(Paragraph::new(tel).style(fg), chunks[0]);
    }

    // Spectrum in middle area
    draw_spectrum(f, chunks[1]);

    // Bottom audio bar
    let (track_name, paused, vol, muted, paused_dur, play_start, paused_state, env_len, env_ready) = match state.lock() {
        Ok(s) => (
            track.to_string(), s.paused, s.volume, s.muted,
            s.paused_duration, s.play_start, s.paused, s.amp_envelope.len(), s.envelope_ready,
        ),
        Err(_) => return,
    };

    let elapsed = if paused_state { paused_dur } else { paused_dur + play_start.elapsed() };
    let elapsed_secs = elapsed.as_secs_f64();

    let dur_secs = if env_ready && env_len > 0 { Some(env_len as f64 * 0.05) } else { None };

    let bar = if let Some(d) = dur_secs {
        let pct = (elapsed_secs / d).clamp(0.0, 1.0);
        let filled = (pct * 10.0).round() as usize;
        let empty = 10usize.saturating_sub(filled);
        format!(" {}", "█".repeat(filled) + &"░".repeat(empty))
    } else { String::new() };

    let dur_str = dur_secs.map(|d| format!("{}:{:02}", (d as u64)/60, (d as u64)%60)).unwrap_or_default();
    let elapsed_str = format!("{}:{:02}", (elapsed_secs as u64)/60, (elapsed_secs as u64)%60);

    let pause_sym = if paused { "⏸" } else { "▶" };
    let mute_sym = if muted { " 🔇" } else { "" };
    let clock = Local::now().format("%H:%M").to_string();

    let status = format!(
        "♫ {}  {}  {:>3.0}%{}{}  {}/{}  {}",
        track_name, pause_sym, vol * 100.0, mute_sym, bar, elapsed_str, dur_str, clock
    );
    f.render_widget(Paragraph::new(status).style(fg), chunks[2]);
}
