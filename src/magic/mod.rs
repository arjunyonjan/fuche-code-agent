mod audio;
pub mod sys;
mod ui;

use std::collections::VecDeque;
use std::io;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use rodio::{OutputStream, Sink};

pub use audio::AudioState;
pub use sys::BIOMARKERS;

pub async fn run() {
    let cfg = crate::config::Config::load();
    let songs = audio::scan_mp3s(&cfg.magic_audio_dir);

    let is_wsl = std::fs::read_to_string("/proc/version")
        .map(|s| s.to_lowercase().contains("microsoft"))
        .unwrap_or(false);

    let (_stream, handle, sink, use_powershell) = if is_wsl {
        (None, None, None, true)
    } else {
        match OutputStream::try_default() {
            Ok((stream, handle)) => {
                let s = Sink::try_new(&handle).ok();
                (Some(stream), Some(handle), s, false)
            }
            Err(_) => (None, None, None, false),
        }
    };

    let state = Arc::new(Mutex::new(AudioState {
        songs,
        current: 0,
        handle,
        sink,
        use_powershell,
        child: None,
        stdin: None,
        volume: cfg.magic_volume,
        muted: cfg.magic_muted,
        dragging: false,
        paused: false,
        paused_duration: Default::default(),
        play_start: Instant::now(),
        amp_envelope: Vec::new(),
        sample_rate: 44100.0,
        envelope_ready: false,
        track_name: String::new(),
    }));
    audio::play_song(&state);

    let mut terminal = ratatui::init();
    crossterm::terminal::enable_raw_mode().ok();
    crossterm::execute!(io::stdout(), crossterm::event::EnableMouseCapture).ok();

    let mut _globe = ui::WireframeGlobe::new(1.0);
    let mut frame_count = 0u64;
    let mut fps_start = Instant::now();
    let mut fps = 0u64;

    let mut prev_cores: Vec<(u64, u64)> = Vec::new();
    let mut core_pcts: Vec<f64> = Vec::new();
    let mut prev_agg = (0u64, 0u64);
    let mut cpu_pct = 0.0f64;
    let mut last_telemetry = Instant::now();
    let mut gpu_temp = 0.0f64;
    let mut gpu_util = 0.0f64;
    let mut gpu_mem_used = 0.0f64;
    let mut gpu_mem_total = 0.0f64;
    let mut ram_used = 0.0f64;
    let mut ram_total = 0.0f64;
    let mut cpu_temp = None;
    let cpu_freq = sys::read_cpu_freq_mhz();
    let cpu_cores = 24;
    let mut sparkline = VecDeque::new();
    let mut esc_pressed = false;

    loop {
        frame_count += 1;
        let elapsed = fps_start.elapsed().as_secs_f64();
        if elapsed >= 0.5 {
            fps = (frame_count as f64 / elapsed).round() as u64;
            frame_count = 0;
            fps_start = Instant::now();
        }

        if last_telemetry.elapsed().as_secs_f64() >= 0.5 {
            let cores = sys::read_core_loads();
            if !prev_cores.is_empty() && cores.len() == prev_cores.len() {
                core_pcts.clear();
                for (i, &(tot, idl)) in cores.iter().enumerate() {
                    let (pt, pi) = prev_cores[i];
                    let dt = tot.saturating_sub(pt);
                    let di = idl.saturating_sub(pi);
                    let p = if dt > 0 { (dt - di) as f64 / dt as f64 * 100.0 } else { 0.0 };
                    core_pcts.push(p);
                }
            } else {
                core_pcts = vec![0.0; cores.len()];
            }
            prev_cores = cores;

            let (agg_tot, agg_idl) = sys::read_cpu_stat();
            if prev_agg.0 > 0 {
                let dt = agg_tot.saturating_sub(prev_agg.0);
                let di = agg_idl.saturating_sub(prev_agg.1);
                cpu_pct = if dt > 0 { (dt - di) as f64 / dt as f64 * 100.0 } else { 0.0 };
            }
            prev_agg = (agg_tot, agg_idl);

            let sv = (cpu_pct * 7.0 / 100.0).round() as u8;
            sparkline.push_back(sv.min(7));
            if sparkline.len() > 120 { sparkline.pop_front(); }

            let (gt, gu, gmu, gmt) = sys::read_gpu_info();
            gpu_temp = gt;
            gpu_util = gu;
            gpu_mem_used = gmu;
            gpu_mem_total = gmt;
            let (ru, rt) = sys::read_memory();
            ram_used = ru;
            ram_total = rt;
            if cpu_temp.is_none() { cpu_temp = sys::read_cpu_temp(); }
            last_telemetry = Instant::now();
        }

        let track = {
            state.lock().ok().map(|s| s.track_name.clone()).unwrap_or_default()
        };

        terminal
            .draw(|f| {
                ui::draw(
                    f, fps, esc_pressed,
                    &state, &track, "",
                    gpu_temp, gpu_util, gpu_mem_used, gpu_mem_total,
                    ram_used, ram_total,
                    cpu_temp, cpu_freq, cpu_cores, &core_pcts, &sparkline,
                );
            })
            .ok();

        if crossterm::event::poll(Duration::from_millis(33)).ok().unwrap_or(false) {
            match crossterm::event::read() {
                Ok(crossterm::event::Event::Key(k)) => {
                    match k.code {
                        crossterm::event::KeyCode::Esc if esc_pressed => break,
                        crossterm::event::KeyCode::Esc => esc_pressed = true,
                        crossterm::event::KeyCode::Char('[') if !esc_pressed => {
                            esc_pressed = false;
                            audio::play_song(&state);
                        }
                        crossterm::event::KeyCode::Char(']') if !esc_pressed => {
                            esc_pressed = false;
                            audio::play_song(&state);
                        }
                        crossterm::event::KeyCode::Char('-') | crossterm::event::KeyCode::Char('_') if !esc_pressed => {
                            let vol = state.lock().ok().map(|s| s.volume).unwrap_or(0.5);
                            audio::set_volume(&state, vol - 0.1);
                        }
                        crossterm::event::KeyCode::Char('=') | crossterm::event::KeyCode::Char('+') if !esc_pressed => {
                            let vol = state.lock().ok().map(|s| s.volume).unwrap_or(0.5);
                            audio::set_volume(&state, vol + 0.1);
                        }
                        crossterm::event::KeyCode::Char(' ') if !esc_pressed => {
                            audio::toggle_pause(&state);
                        }
                        crossterm::event::KeyCode::Char('m') | crossterm::event::KeyCode::Char('M') if !esc_pressed => {
                            audio::toggle_mute(&state);
                        }
                        _ => esc_pressed = false,
                    }
                }
                Ok(crossterm::event::Event::Mouse(m)) => {
                    let (term_w, term_h) = crossterm::terminal::size().unwrap_or((80, 24));
                    let vol_row = term_h.saturating_sub(2);
                    let sec_x = (term_w as i16 - 29).max(0) as u16;
                    let pause_x = sec_x;
                    let bar_x = sec_x + 4;
                    let bar_w = 13;
                    let down_x = sec_x + 22;
                    let up_x = sec_x + 24;
                    match m.kind {
                        crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                            if m.row == vol_row && m.column >= bar_x && m.column < bar_x + bar_w {
                                let pct = ((m.column - bar_x) as f64 / bar_w as f64).clamp(0.0, 1.0);
                                audio::set_volume(&state, pct);
                                if let Ok(mut s) = state.lock() { s.dragging = true; }
                            } else if m.row == vol_row && m.column >= pause_x && m.column < pause_x + 2 {
                                audio::toggle_pause(&state);
                            } else if m.row == vol_row && m.column >= down_x && m.column < down_x + 2 {
                                if let Ok(mut s) = state.lock() { s.volume = (s.volume - 0.1).clamp(0.0, 1.0); }
                            } else if m.row == vol_row && m.column >= up_x && m.column < up_x + 2 {
                                if let Ok(mut s) = state.lock() { s.volume = (s.volume + 0.1).clamp(0.0, 1.0); }
                            }
                        }
                        crossterm::event::MouseEventKind::Drag(crossterm::event::MouseButton::Left) => {
                            let dragging = state.lock().ok().map(|s| s.dragging).unwrap_or(false);
                            if dragging && m.column >= bar_x && m.column < bar_x + bar_w {
                                let pct = ((m.column - bar_x) as f64 / bar_w as f64).clamp(0.0, 1.0);
                                audio::set_volume(&state, pct);
                            }
                        }
                        crossterm::event::MouseEventKind::Up(_) => {
                            if let Ok(mut s) = state.lock() { s.dragging = false; }
                        }
                        _ => {}
                    }
                }
                _ => esc_pressed = false,
            }
        }
        // auto-advance when PowerShell child exits (MediaEnded → add_MediaEnded({exit}))
        if let Ok(mut s) = state.lock() {
            if s.use_powershell && !s.paused {
                let ended = s.child.as_mut().and_then(|c| c.try_wait().ok()).flatten().is_some();
                if ended {
                    s.child = None;
                    s.stdin = None;
                    drop(s);
                    audio::play_song(&state);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(16));
    }

    if let Ok(mut s) = state.lock() {
        if s.use_powershell {
            if let Some(mut child) = s.child.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        } else if let Some(ref sink) = s.sink {
            sink.stop();
        }
        let mut cfg = crate::config::Config::load();
        cfg.magic_volume = s.volume;
        cfg.magic_muted = s.muted;
        cfg.save();
    }

    crossterm::execute!(io::stdout(), crossterm::event::DisableMouseCapture).ok();
    crossterm::terminal::disable_raw_mode().ok();
    ratatui::restore();
}
