use std::io::BufReader;
use std::io::Write;
use std::process::Child;
use std::process::ChildStdin;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;
use rodio::{Decoder, OutputStreamHandle, Sink};

pub struct AudioState {
    pub songs: Vec<String>,
    pub current: usize,
    pub handle: Option<OutputStreamHandle>,
    pub sink: Option<Sink>,
    pub use_powershell: bool,
    pub child: Option<Child>,
    pub stdin: Option<ChildStdin>,
    pub volume: f64,
    pub muted: bool,
    pub dragging: bool,
    pub paused: bool,
    pub play_start: Instant,
    pub amp_envelope: Vec<f64>,
    pub sample_rate: f64,
    pub envelope_ready: bool,
    pub track_name: String,
}

pub fn scan_mp3s(dir: &str) -> Vec<String> {
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

fn wsl_to_win(path: &str) -> String {
    let trimmed = path.trim_start_matches("/mnt/");
    let drive = trimmed.chars().next()
        .map(|c| format!("{}:", c.to_ascii_uppercase()))
        .unwrap_or_default();
    let rest = trimmed.chars().skip(1).collect::<String>();
    format!("{}{}", drive, rest.replace('/', "\\"))
}

pub fn set_volume(state: &Arc<Mutex<AudioState>>, vol: f64) {
    if let Ok(mut s) = state.lock() {
        s.volume = vol.clamp(0.0, 1.0);
        if s.use_powershell {
            let v = s.volume;
            if let Some(ref mut stdin) = s.stdin {
                let _ = writeln!(stdin, "volume:{}", v);
            }
        }
    }
}

pub fn toggle_pause(state: &Arc<Mutex<AudioState>>) {
    if let Ok(mut s) = state.lock() {
        s.paused = !s.paused;
        let cmd = if s.paused { "pause" } else { "play" };
        if s.use_powershell {
            if let Some(ref mut stdin) = s.stdin {
                let _ = writeln!(stdin, "{}", cmd);
            }
        } else if let Some(ref sink) = s.sink {
            if s.paused { sink.pause(); } else { sink.play(); }
        }
    }
}

pub fn toggle_mute(state: &Arc<Mutex<AudioState>>) {
    if let Ok(mut s) = state.lock() {
        s.muted = !s.muted;
        let m = s.muted;
        if s.use_powershell {
            if let Some(ref mut stdin) = s.stdin {
                let _ = writeln!(stdin, "mute:{}", m);
            }
        }
    }
}

pub fn play_song(state: &Arc<Mutex<AudioState>>) {
    let Ok(mut s) = state.lock() else { return };
    if s.songs.is_empty() { return; }
    if s.songs.len() > 1 {
        let next = pick_random(s.songs.len(), Some(s.current));
        s.current = next;
    }
    let idx = s.current;
    drop(s);
    play_idx(state, idx);
}

pub fn play_idx(state: &Arc<Mutex<AudioState>>, idx: usize) {
    let Ok(mut s) = state.lock() else { return };
    if s.songs.is_empty() { return; }
    s.current = idx.min(s.songs.len() - 1);
    let path = s.songs[s.current].clone();
    s.track_name = extract_name(&path);

    s.paused = false;
    if s.use_powershell {
        if let Some(mut child) = s.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let win = wsl_to_win(&path);
        let vol = s.volume;
        let m = s.muted;
        let cmd = format!(
            "Add-Type -AssemblyName PresentationCore; \
             $p = New-Object System.Windows.Media.MediaPlayer; \
             $p.Open('{}'); $p.Volume = {:.2}; $p.IsMuted = ${}; $p.Play(); \
             while($true){{$l=[Console]::In.ReadLine();if($l-eq$null){{break}}\
             elseif($l-eq'pause'){{$p.Pause()}}elseif($l-eq'play'){{$p.Play()}}\
             elseif($l-match'^volume:(.+)$'){{$p.Volume=[double]::Parse($matches[1])}}\
             elseif($l-match'^mute:(.+)$'){{$p.IsMuted=[bool]::Parse($matches[1])}}\
             elseif($l-eq'stop'){{exit}}}}", win, vol, if m { "true" } else { "false" });
        use std::process::Stdio;
        if let Ok(mut child) = std::process::Command::new("powershell.exe")
            .stdin(Stdio::piped())
            .args(["-WindowStyle", "Hidden", "-NoProfile", "-Command", &cmd]).spawn()
        {
            s.stdin = child.stdin.take();
            s.child = Some(child);
        }
    } else if let Some(ref handle) = s.handle {
        if let Ok(new_sink) = Sink::try_new(handle) {
            if let Ok(file) = std::fs::File::open(&path) {
                if let Ok(source) = Decoder::new(BufReader::new(file)) {
                    new_sink.append(source);
                    s.sink = Some(new_sink);
                }
            }
        }
    }

    s.play_start = Instant::now();
    s.envelope_ready = false;
    s.amp_envelope.clear();
    let state2 = state.clone();
    std::thread::spawn(move || {
        if let Some((env, sr)) = decode_envelope(&path) {
            if let Ok(mut st) = state2.lock() {
                st.amp_envelope = env;
                st.sample_rate = sr;
                st.envelope_ready = true;
            }
        }
    });
}
