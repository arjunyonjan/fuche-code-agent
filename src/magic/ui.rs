#![allow(dead_code)]

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{canvas::{Line as CLine, Painter, Shape}, Paragraph},
    Frame,
};

use crate::magic::audio::AudioState;

pub(super) struct WireframeGlobe {
    segments: Vec<[(f64, f64, f64); 2]>,
    rotation: f64,
}

impl WireframeGlobe {
    #[allow(dead_code)]
    pub(super) fn new(radius: f64) -> Self {
        let mut segments = Vec::new();
        let lat_circle_segs = 48;
        let meridian_count = 24;
        let meridian_steps = 32;
        for i in 0..11 {
            let lat_deg = -75.0 + (i as f64) * 15.0;
            let lat = lat_deg.to_radians();
            let r = radius * lat.cos();
            let y = radius * lat.sin();
            for j in 0..lat_circle_segs {
                let a1 = (j as f64) * 2.0 * std::f64::consts::PI / lat_circle_segs as f64;
                let a2 = ((j + 1) as f64) * 2.0 * std::f64::consts::PI / lat_circle_segs as f64;
                segments.push([(r * a1.cos(), y, r * a1.sin()), (r * a2.cos(), y, r * a2.sin())]);
            }
        }
        for i in 0..meridian_count {
            let lon = (i as f64) * 2.0 * std::f64::consts::PI / meridian_count as f64;
            let (lon_sin, lon_cos) = lon.sin_cos();
            for j in 0..meridian_steps {
                let lat1 = (-75.0 + (j as f64) * 150.0 / meridian_steps as f64).to_radians();
                let lat2 = (-75.0 + ((j + 1) as f64) * 150.0 / meridian_steps as f64).to_radians();
                segments.push([
                    (radius * lat1.cos() * lon_cos, radius * lat1.sin(), radius * lat1.cos() * lon_sin),
                    (radius * lat2.cos() * lon_cos, radius * lat2.sin(), radius * lat2.cos() * lon_sin),
                ]);
            }
        }
        Self { segments, rotation: 0.0 }
    }
}

impl Shape for WireframeGlobe {
    fn draw(&self, painter: &mut Painter) {
        let (sin_a, cos_a) = self.rotation.sin_cos();
        let (&[left, right], &[bottom, top]) = painter.bounds();
        let steps = (right - left).min(top - bottom) * 60.0;
        let light_len = (-0.4_f64 * -0.4 + 0.8 * 0.8 + 0.5 * 0.5).sqrt();
        let (lx, ly, lz) = (-0.4 / light_len, 0.8 / light_len, 0.5 / light_len);
        for y in 0..=steps as usize {
            for x in 0..=steps as usize {
                let wx = left + (right - left) * x as f64 / steps as f64;
                let wy = bottom + (top - bottom) * y as f64 / steps as f64;
                let r2 = wx * wx + wy * wy;
                if r2 > 1.0 { continue; }
                let wz = (1.0 - r2).sqrt();
                let dot = (wx * cos_a + wz * sin_a) * lx + wy * ly + (-wx * sin_a + wz * cos_a) * lz;
                let b = (dot * 0.6 + 0.3).clamp(0.0, 1.0);
                if let Some((gx, gy)) = painter.get_point(wx, wy) {
                    let g = (230.0 * b) as u8;
                    let bv = (200.0 * b) as u8;
                    painter.paint(gx, gy, Color::Rgb(0, g, bv));
                }
            }
        }
        for &[(x1, y1, z1), (x2, y2, z2)] in &self.segments {
            let rx1 = x1 * cos_a + z1 * sin_a;
            let ry1 = y1;
            let rz1 = -x1 * sin_a + z1 * cos_a;
            let rx2 = x2 * cos_a + z2 * sin_a;
            let ry2 = y2;
            let mz = (rz1 + (-x2 * sin_a + z2 * cos_a)) / 2.0;
            let dim = if mz > 0.0 { 1.0 } else { 0.3 };
            let g = (230.0 * dim) as u8;
            let bv = (200.0 * dim) as u8;
            CLine::new(rx1, ry1, rx2, ry2, Color::Rgb(0, g, bv)).draw(painter);
        }
    }
}

const SPARK: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

fn heat_color(pct: f64) -> Color {
    if pct < 25.0 { Color::Rgb(0, 180, 120) }
    else if pct < 50.0 { Color::Rgb(100, 230, 100) }
    else if pct < 75.0 { Color::Rgb(255, 220, 50) }
    else if pct < 90.0 { Color::Rgb(255, 150, 0) }
    else { Color::Rgb(255, 50, 50) }
}

fn block_bar(pct: f64, width: usize) -> String {
    let filled = (pct * width as f64).round() as usize;
    let filled = filled.min(width);
    std::iter::repeat('█').take(filled)
        .chain(std::iter::repeat('░').take(width.saturating_sub(filled)))
        .collect()
}

fn bio_bar(val: f64, max: f64, invert: bool) -> String {
    let pct = if invert { (1.0 - val / max).clamp(0.0, 1.0) } else { (val / max).clamp(0.0, 1.0) };
    let filled = (pct * 10.0).round() as usize;
    let bar: String = std::iter::repeat('█').take(filled)
        .chain(std::iter::repeat('░').take(10 - filled))
        .collect();
    format!("{} {:>3}%", bar, (pct * 100.0).round() as u8)
}

pub fn draw(
    f: &mut Frame,
    _fps: u64,
    esc_pressed: bool,
    state: &Arc<Mutex<AudioState>>,
    track: &str,
    _last_key: &str,
    _gpu_temp: f64,
    _gpu_util: f64,
    _gpu_mem_used: f64,
    _gpu_mem_total: f64,
    _ram_used: f64,
    _ram_total: f64,
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
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(f.area());

    let (track_name, paused, vol, muted) = match state.lock() {
        Ok(s) => (track.to_string(), s.paused, s.volume, s.muted),
        Err(_) => return,
    };

    let pause_sym = if paused { "⏸" } else { "▶" };
    let mute_sym = if muted { "  🔇" } else { "" };
    let status = format!(
        "♫ {}   {}  {:>3.0}%{}     [ ]skip  -=vol  m mute  Space pause",
        track_name, pause_sym, vol * 100.0, mute_sym
    );
    f.render_widget(Paragraph::new(status).style(fg), chunks[1]);
}
