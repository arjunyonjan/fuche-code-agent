use std::sync::LazyLock;
use std::sync::Mutex;

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

pub fn read_core_loads() -> Vec<(u64, u64)> {
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

pub fn read_gpu_info() -> (f64, f64, f64, f64) {
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

pub fn read_cpu_stat() -> (u64, u64) {
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

pub fn read_memory() -> (f64, f64) {
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

pub fn read_cpu_freq_mhz() -> f64 {
    let content = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    content.lines().find(|l| l.contains("cpu MHz"))
        .and_then(|l| l.split(':').nth(1))
        .and_then(|s| s.trim().parse::<f64>().ok())
        .unwrap_or(0.0)
}



pub fn read_cpu_temp() -> Option<f64> {
    std::fs::read_to_string("/sys/class/thermal/thermal_zone0/temp")
        .ok()
        .and_then(|s| s.trim().parse::<f64>().ok())
        .map(|t| t / 1000.0)
        .or_else(|| {
            std::fs::read_to_string("/sys/class/hwmon/hwmon0/temp1_input")
                .ok()
                .and_then(|s| s.trim().parse::<f64>().ok())
                .map(|t| t / 1000.0)
        })
}
