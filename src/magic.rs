use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Line as CLine, Painter, Shape},
        Block, Borders, Paragraph,
    },
};
use std::collections::VecDeque;
use std::io;
use std::process::Child;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct Biomarkers {
    pub sleep_hours: f64,
    pub hydration_l: f64,
    pub vitamin_d: f64,
    pub cortisol: f64,
}

impl Default for Biomarkers {
    fn default() -> Self {
        Self { sleep_hours: 0.0, hydration_l: 0.0, vitamin_d: 0.0, cortisol: 0.0 }
    }
}

pub static BIOMARKERS: LazyLock<Mutex<Biomarkers>> = LazyLock::new(|| Mutex::new(Biomarkers::default()));

struct WireframeGlobe {
    segments: Vec<[(f64, f64, f64); 2]>,
    rotation: f64,
}

impl WireframeGlobe {
    fn new(radius: f64) -> Self {
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

fn read_core_loads() -> Vec<(u64, u64)> {
    let content = std::fs::read_to_string("/proc/stat").unwrap_or_default();
    let mut cores = Vec::new();
    for line in content.lines().skip(1) {
        if !line.starts_with("cpu") { break; }
        let parts: Vec<u64> = line.split_whitespace().skip(1)
            .filter_map(|s| s.parse::<u64>().ok()).collect();
        if parts.len() >= 5 {
            let total: u64 = parts.iter().sum();
            cores.push((total, parts[3] + parts[4]));
        }
    }
    cores
}

fn read_gpu_info() -> (f64, f64, f64, f64) {
    let out = std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=temperature.gpu,utilization.gpu,memory.used,memory.total", "--format=csv,noheader,nounits"])
        .output().ok();
    match out {
        Some(o) if o.status.success() => {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let parts: Vec<f64> = s.split(',').filter_map(|p| p.trim().parse::<f64>().ok()).collect();
            if parts.len() >= 4 { (parts[0], parts[1], parts[2], parts[3]) } else { (0.0, 0.0, 0.0, 0.0) }
        }
        _ => (0.0, 0.0, 0.0, 0.0),
    }
}

fn block_bar(pct: f64, width: usize) -> String {
    let filled = (pct * width as f64).round() as usize;
    let filled = filled.min(width);
    std::iter::repeat('█').take(filled)
        .chain(std::iter::repeat('░').take(width.saturating_sub(filled)))
        .collect()
}

fn read_cpu_stat() -> (u64, u64) {
    let content = std::fs::read_to_string("/proc/stat").unwrap_or_default();
    let parts: Vec<u64> = content
        .lines().next().unwrap_or_default()
        .split_whitespace().skip(1)
        .filter_map(|s| s.parse::<u64>().ok())
        .collect();
    let total: u64 = parts.iter().sum();
    let idle = parts.get(3).copied().unwrap_or(0) + parts.get(4).copied().unwrap_or(0);
    (total, idle)
}

fn read_memory() -> (f64, f64) {
    let content = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut total = 0.0f64;
    let mut avail = 0.0f64;
    for line in content.lines() {
        if line.starts_with("MemTotal:") {
            total = line.split_whitespace().nth(1).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
        }
        if line.starts_with("MemAvailable:") {
            avail = line.split_whitespace().nth(1).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
        }
    }
    ((total - avail) / 1_048_576.0, total / 1_048_576.0)
}

fn read_cpu_freq_mhz() -> f64 {
    let content = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    content.lines().find(|l| l.contains("cpu MHz"))
        .and_then(|l| l.split(':').nth(1))
        .and_then(|s| s.trim().parse::<f64>().ok())
        .unwrap_or(0.0)
}

fn read_cpu_temp() -> Option<f64> {
    let s = std::fs::read_to_string("/sys/class/thermal/thermal_zone0/temp").ok()?;
    let t = s.trim().parse::<f64>().ok()?;
    Some(t / 1000.0)
}

fn bio_bar(val: f64, max: f64, invert: bool) -> String {
    let pct = if invert { (1.0 - val / max).clamp(0.0, 1.0) } else { (val / max).clamp(0.0, 1.0) };
    let filled = (pct * 10.0).round() as usize;
    let bar: String = std::iter::repeat('█').take(filled)
        .chain(std::iter::repeat('░').take(10 - filled))
        .collect();
    format!("{} {:>3}%", bar, (pct * 100.0).round() as u8)
}

struct AudioState {
    songs: Vec<String>,
    current: usize,
    child: Option<Child>,
    play_start: Instant,
    amp_envelope: Vec<f64>,
    sample_rate: f64,
    envelope_ready: bool,
    track_name: String,
}

fn scan_mp3s() -> Vec<String> {
    let dir = "/mnt/c/Users/ACER/Downloads/Music/ACDC";
    let mut songs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "mp3").unwrap_or(false) {
                songs.push(path.to_string_lossy().to_string());
            }
        }
    }
    songs.sort();
    songs
}

fn wsl_to_win(wsl: &str) -> String {
    // Try wslpath first
    if let Ok(out) = std::process::Command::new("wslpath")
        .args(["-w", wsl]).output()
    {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() { return s; }
        }
    }
    // Fallback: /mnt/c/... → C:\...
    let s = wsl.trim_start_matches("/mnt/");
    let drive = s.chars().next().map(|c| format!("{}:", c.to_ascii_uppercase())).unwrap_or_default();
    let rest = s.chars().skip(1).collect::<String>();
    format!("{}{}", drive, rest.replace('/', "\\"))
}

fn pick_random(len: usize, exclude: Option<usize>) -> usize {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap()
        .subsec_nanos() as usize;
    let mut idx = nanos % len;
    if let Some(ex) = exclude {
        if len > 1 && idx == ex { idx = (idx + 1) % len; }
    }
    idx
}

fn decode_envelope(path: &str) -> Option<(Vec<f64>, f64)> {
    use rodio::Source;
    let file = std::fs::File::open(path).ok()?;
    let source = rodio::Decoder::new(std::io::BufReader::new(file)).ok()?;
    let sample_rate = source.sample_rate() as f64;
    let window = (sample_rate * 0.05) as usize;
    let samples: Vec<f32> = source.convert_samples::<f32>().collect();
    let mut envelope = Vec::with_capacity(samples.len() / window + 1);
    for chunk in samples.chunks(window) {
        if chunk.is_empty() { continue; }
        let rms = (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32).sqrt();
        envelope.push(rms as f64);
    }
    let max = envelope.iter().fold(0.0f64, |a, &b| a.max(b));
    if max > 0.0 { for v in &mut envelope { *v /= max; } }
    Some((envelope, sample_rate))
}

fn extract_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem().and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string()
}

fn play_song(state: &Arc<Mutex<AudioState>>) {
    let mut s = state.lock().unwrap();
    if let Some(mut child) = s.child.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    if s.songs.len() > 1 {
        let next = pick_random(s.songs.len(), Some(s.current));
        s.current = next;
    }
    let path = s.songs[s.current].clone();
    s.track_name = extract_name(&path);
    let win = wsl_to_win(&path);
    eprintln!("  ♫ Now playing [{}]: {}", s.current, s.track_name);
    let cmd = format!(
        "Add-Type -AssemblyName PresentationCore; \
         $p = New-Object System.Windows.Media.MediaPlayer; \
         $p.Open('{}'); $p.Play(); Start-Sleep 9999", win);
    if let Ok(c) = std::process::Command::new("powershell.exe")
        .args(["-Command", &cmd]).spawn()
    { s.child = Some(c); }
    else { eprintln!("  ❌ Failed to spawn powershell.exe"); }
    s.play_start = Instant::now();
    s.envelope_ready = false;
    s.amp_envelope.clear();
    let state2 = state.clone();
    std::thread::spawn(move || {
        if let Some((env, sr)) = decode_envelope(&path) {
            let mut st = state2.lock().unwrap();
            st.amp_envelope = env;
            st.sample_rate = sr;
            st.envelope_ready = true;
        }
    });
}

pub async fn run() {
    let songs = scan_mp3s();
    if songs.is_empty() {
        eprintln!("No MP3s found in ACDC folder");
        return;
    }
    let state = Arc::new(Mutex::new(AudioState {
        songs,
        current: 0, // play_song picks random on first call
        child: None,
        play_start: Instant::now(),
        amp_envelope: Vec::new(),
        sample_rate: 44100.0,
        envelope_ready: false,
        track_name: String::new(),
    }));
    play_song(&state);

    let mut terminal = ratatui::init();
    crossterm::terminal::enable_raw_mode().ok();
    crossterm::execute!(io::stdout(), crossterm::event::EnableMouseCapture).ok();

    let mut globe = WireframeGlobe::new(1.0);
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
    let cpu_freq = read_cpu_freq_mhz();
    let cpu_cores = 24;
    let mut sparkline = VecDeque::new();
    let mut esc_pressed = false;

    loop {
        globe.rotation += 0.015;

        frame_count += 1;
        let elapsed = fps_start.elapsed().as_secs_f64();
        if elapsed >= 0.5 {
            fps = (frame_count as f64 / elapsed).round() as u64;
            frame_count = 0;
            fps_start = Instant::now();
        }

        if last_telemetry.elapsed().as_secs_f64() >= 0.5 {
            let cores = read_core_loads();
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

            let (agg_tot, agg_idl) = read_cpu_stat();
            if prev_agg.0 > 0 {
                let dt = agg_tot.saturating_sub(prev_agg.0);
                let di = agg_idl.saturating_sub(prev_agg.1);
                cpu_pct = if dt > 0 { (dt - di) as f64 / dt as f64 * 100.0 } else { 0.0 };
            }
            prev_agg = (agg_tot, agg_idl);

            let sv = (cpu_pct * 7.0 / 100.0).round() as u8;
            sparkline.push_back(sv.min(7));
            if sparkline.len() > 120 { sparkline.pop_front(); }

            let (gt, gu, gmu, gmt) = read_gpu_info();
            gpu_temp = gt;
            gpu_util = gu;
            gpu_mem_used = gmu;
            gpu_mem_total = gmt;
            let (ru, rt) = read_memory();
            ram_used = ru;
            ram_total = rt;
            if cpu_temp.is_none() { cpu_temp = read_cpu_temp(); }
            last_telemetry = Instant::now();
        }

        let audio_state = state.lock().unwrap();
        let track = audio_state.track_name.clone();
        let env_ready = audio_state.envelope_ready;
        let env = audio_state.amp_envelope.clone();
        let env_sr = audio_state.sample_rate;
        let play_time = audio_state.play_start.elapsed().as_secs_f64();
        drop(audio_state);

        terminal
            .draw(|f| {
                let area = f.area();
                let title = if fps > 0 {
                    format!(" MAGIC MODE  ·  {fps} FPS ")
                } else {
                    " MAGIC MODE ".to_string()
                };
                let block = Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(0, 230, 200)))
                    .style(Style::default().bg(Color::Black));
                let inner = block.inner(area);
                f.render_widget(block, area);

                let vert = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(2)])
                    .split(inner);

                let horiz = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                    .split(vert[0]);

                let w = horiz[0].width as f64;
                let h = horiz[0].height as f64;
                let y_half = 1.3;
                let x_half = y_half * w / (2.0 * h);

                let canvas = Canvas::default()
                    .x_bounds([-x_half, x_half])
                    .y_bounds([-y_half, y_half])
                    .marker(Marker::Braille)
                    .background_color(Color::Black)
                    .paint(|ctx| ctx.draw(&globe));
                f.render_widget(canvas, horiz[0]);

                let side = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(8)])
                    .split(horiz[1]);

                let fg = Style::default().fg(Color::Rgb(0, 230, 200));
                let bp = Style::default().fg(Color::Rgb(0, 230, 200)).bg(Color::Black);

                // ── TELEMETRY outer panel ──
                let tele = Block::default()
                    .title(" TELEMETRY ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(0, 230, 200)))
                    .style(Style::default().bg(Color::Black));
                let tele_inner = tele.inner(side[0]);
                f.render_widget(tele, side[0]);

                let tchunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(4), Constraint::Min(1), Constraint::Length(3)])
                    .split(tele_inner);

                // ── GPU + VRAM row ──
                let gpuchunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(tchunks[0]);

                let gpu_block = Block::default()
                    .title(" GPU ").borders(Borders::ALL).border_style(bp).style(bp);
                let gpu_inner = gpu_block.inner(gpuchunks[0]);
                f.render_widget(gpu_block, gpuchunks[0]);
                let gu_bar = block_bar(gpu_util / 100.0, gpu_inner.width as usize);
                let mut gt = ratatui::text::Text::default();
                gt.push_line(Line::from(format!("{:>3}°C", gpu_temp as u64)));
                gt.push_line(Line::from(format!("{} {:>3.0}%", gu_bar, gpu_util)));
                f.render_widget(Paragraph::new(gt).style(fg), gpu_inner);

                let vram_block = Block::default()
                    .title(" VRAM ").borders(Borders::ALL).border_style(bp).style(bp);
                let vram_inner = vram_block.inner(gpuchunks[1]);
                f.render_widget(vram_block, gpuchunks[1]);
                let vm_pct = if gpu_mem_total > 0.0 { gpu_mem_used / gpu_mem_total * 100.0 } else { 0.0 };
                let vm_bar = block_bar(vm_pct / 100.0, vram_inner.width as usize);
                let mut vt = ratatui::text::Text::default();
                vt.push_line(Line::from(format!("{:.0}M/{:.0}G", gpu_mem_used, gpu_mem_total / 1000.0)));
                vt.push_line(Line::from(format!("{} {:>3.0}%", vm_bar, vm_pct)));
                f.render_widget(Paragraph::new(vt).style(fg), vram_inner);

                // ── CPU panel ──
                let cpu_block = Block::default()
                    .title(" CPU ").borders(Borders::ALL).border_style(bp).style(bp);
                let cpu_inner = cpu_block.inner(tchunks[1]);
                f.render_widget(cpu_block, tchunks[1]);

                let cpu_temp_str = cpu_temp.map(|t| format!("{:>3}°C", t as u64)).unwrap_or_else(|| " --°C".to_string());
                let freq_str = if cpu_freq > 0.0 { format!("{:.2}GHz", cpu_freq / 1000.0) } else { " --".to_string() };
                let mut cpu_text = ratatui::text::Text::default();
                cpu_text.push_line(Line::from(format!("{}  {}c @ {}", cpu_temp_str, cpu_cores, freq_str)));

                let ciw = cpu_inner.width as usize;
                let ncols = 12.min(ciw.saturating_sub(1));
                if !core_pcts.is_empty() {
                    for row in 0..(core_pcts.len() + ncols - 1) / ncols {
                        let mut spans = Vec::new();
                        let start = row * ncols;
                        for &p in core_pcts[start..start + ncols.min(core_pcts.len() - start)].iter() {
                            let idx = (p * 7.0 / 100.0).round() as usize;
                            spans.push(Span::styled(SPARK[idx.min(7)].to_string(), Style::default().fg(heat_color(p))));
                        }
                        cpu_text.push_line(Line::from(spans));
                    }
                }

                let max_spark = ciw.saturating_sub(1).min(sparkline.len());
                if max_spark > 0 {
                    let mut spans = Vec::new();
                    let start = sparkline.len() - max_spark;
                    for &v in sparkline.range(start..) {
                        spans.push(Span::styled(SPARK[v as usize].to_string(), fg));
                    }
                    cpu_text.push_line(Line::from(spans));
                }
                f.render_widget(Paragraph::new(cpu_text).style(fg), cpu_inner);

                // ── RAM panel ──
                let ram_block = Block::default()
                    .title(" RAM ").borders(Borders::ALL).border_style(bp).style(bp);
                let ram_inner = ram_block.inner(tchunks[2]);
                f.render_widget(ram_block, tchunks[2]);

                let ram_pct = if ram_total > 0.0 { ram_used / ram_total * 100.0 } else { 0.0 };
                let ram_bar = block_bar(ram_pct / 100.0, ram_inner.width as usize);
                let mut ramt = ratatui::text::Text::default();
                ramt.push_line(Line::from(format!("{:.1}G/{:.0}G", ram_used, ram_total)));
                ramt.push_line(Line::from(format!("{} {:>3.0}%", ram_bar, ram_pct)));
                f.render_widget(Paragraph::new(ramt).style(fg), ram_inner);

                // ── BIOMARKERS panel ──
                let bio = BIOMARKERS.lock().unwrap().clone();
                let bio_block = Block::default()
                    .title(" BIOMARKERS ").borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(0, 230, 200)))
                    .style(Style::default().bg(Color::Black));
                let bio_inner = bio_block.inner(side[1]);
                f.render_widget(bio_block, side[1]);

                let mut bio_text = ratatui::text::Text::default();
                let bfg = Style::default().fg(Color::Rgb(0, 230, 200));
                bio_text.push_line(Line::from(format!("Sleep {:.1}h {}", bio.sleep_hours, bio_bar(bio.sleep_hours, 10.0, false))));
                bio_text.push_line(Line::from(format!("Water {:.1}L {}", bio.hydration_l, bio_bar(bio.hydration_l, 4.0, false))));
                bio_text.push_line(Line::from(format!("Vit D {:.0}ng {}", bio.vitamin_d, bio_bar(bio.vitamin_d, 100.0, false))));
                bio_text.push_line(Line::from(format!("Cort {:.0}nM {}", bio.cortisol, bio_bar(bio.cortisol, 30.0, true))));
                f.render_widget(Paragraph::new(bio_text).style(bfg), bio_inner);

                // ── bottom area: two rows ──
                let bchunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(1), Constraint::Length(1)])
                    .split(vert[1]);

                if esc_pressed {
                    let confirm = Paragraph::new("Press Esc again to exit, or any other key to continue")
                        .style(Style::default().fg(Color::Rgb(255, 200, 0)))
                        .alignment(Alignment::Center);
                    f.render_widget(confirm, bchunks[0]);
                } else {
                    let info = format!("Jarvis Online ⚡  ·  ♫ {}    < > skip", track);
                    let bottom = Paragraph::new(info)
                        .style(Style::default().fg(Color::Rgb(0, 230, 200)))
                        .alignment(Alignment::Center);
                    f.render_widget(bottom, bchunks[0]);
                }

                if env_ready && !env.is_empty() {
                    let bw = bchunks[1].width as usize;
                    let env_idx = (play_time * env_sr / 50.0) as usize;
                    let half = bw / 2;
                    let start = env_idx.saturating_sub(half);
                    let end = (start + bw).min(env.len());
                    let start = end.saturating_sub(bw);
                    let mut wav_spans = Vec::with_capacity(bw);
                    for i in start..end {
                        let idx = (env[i] * 7.0).round() as usize;
                        wav_spans.push(Span::styled(SPARK[idx.min(7)].to_string(), fg));
                    }
                    let wav_line = Paragraph::new(Line::from(wav_spans))
                        .style(fg)
                        .alignment(Alignment::Center);
                    f.render_widget(wav_line, bchunks[1]);
                }
            })
            .ok();

        if crossterm::event::poll(Duration::from_millis(33)).ok().unwrap_or(false) {
            if let Ok(crossterm::event::Event::Key(k)) = crossterm::event::read() {
                match k.code {
                    crossterm::event::KeyCode::Esc if esc_pressed => break,
                    crossterm::event::KeyCode::Esc => esc_pressed = true,
                    crossterm::event::KeyCode::Char(',') | crossterm::event::KeyCode::Char('<') => {
                        esc_pressed = false;
                        play_song(&state);
                    }
                    crossterm::event::KeyCode::Char('.') | crossterm::event::KeyCode::Char('>') => {
                        esc_pressed = false;
                        play_song(&state);
                    }
                    _ => esc_pressed = false,
                }
            }
        }
    }

    // Kill audio on exit
    if let Ok(mut s) = state.lock() {
        if let Some(mut child) = s.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    crossterm::execute!(io::stdout(), crossterm::event::DisableMouseCapture).ok();
    crossterm::terminal::disable_raw_mode().ok();
    ratatui::restore();
}
