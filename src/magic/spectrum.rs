use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub fn start_spectrum() -> mpsc::Receiver<Vec<f32>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let host = cpal::default_host();
        let device = host.default_input_device().expect("no input device");
        let config = device.default_input_config().unwrap();
        let sample_rate = config.sample_rate().0 as f32;
        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _| {
                // crude magnitude per chunk (replace with real FFT later)
                let mag = data.iter().map(|&s| s.abs()).sum::<f32>() / data.len() as f32;
                let bars = (0..16).map(|i| mag * (1.0 - i as f32 / 16.0)).collect();
                let _ = tx.send(bars);
            },
            |err| eprintln!("audio error: {}", err),
            None,
        ).unwrap();
        stream.play().unwrap();
        loop { thread::sleep(Duration::from_millis(50)); }
    });
    rx
}

pub fn bars_to_ascii(magnitudes: &[f32]) -> String {
    let chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    magnitudes.iter()
        .map(|&m| {
            let idx = (m * 7.0).clamp(0.0, 7.0) as usize;
            chars[idx]
        })
        .collect()
}
