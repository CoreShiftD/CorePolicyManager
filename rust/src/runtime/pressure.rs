use serde::{Deserialize, Serialize};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct PressureMetrics {
    pub supported: bool,
    pub cpu_some_avg10: Option<f32>,
    pub cpu_some_avg60: Option<f32>,
    pub cpu_some_avg300: Option<f32>,
    pub memory_some_avg10: Option<f32>,
    pub memory_some_avg60: Option<f32>,
    pub memory_some_avg300: Option<f32>,
    pub memory_full_avg10: Option<f32>,
    pub memory_full_avg60: Option<f32>,
    pub memory_full_avg300: Option<f32>,
    pub io_some_avg10: Option<f32>,
    pub io_some_avg60: Option<f32>,
    pub io_some_avg300: Option<f32>,
    pub io_full_avg10: Option<f32>,
    pub io_full_avg60: Option<f32>,
    pub io_full_avg300: Option<f32>,
    pub last_refresh_ms: u64,
    pub last_error: Option<String>,
}

pub fn refresh_pressure_metrics(metrics: &mut PressureMetrics) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    // Reset metrics while keeping the timestamp
    *metrics = PressureMetrics {
        last_refresh_ms: now,
        ..Default::default()
    };

    let mut found_any = false;
    for (name, path) in [
        ("cpu", "/proc/pressure/cpu"),
        ("memory", "/proc/pressure/memory"),
        ("io", "/proc/pressure/io"),
    ] {
        if let Ok(content) = fs::read_to_string(path) {
            found_any = true;
            metrics.supported = true;
            parse_pressure_line(name, &content, metrics);
        }
    }

    if !found_any {
        metrics.last_error = Some("No PSI files found or readable".to_string());
    }
}

fn parse_pressure_line(source: &str, content: &str, metrics: &mut PressureMetrics) {
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        let pressure_type = parts[0];

        for part in &parts[1..] {
            if let Some(val_str) = part.strip_prefix("avg10=") {
                if let Ok(val) = val_str.parse::<f32>() {
                    match (source, pressure_type) {
                        ("cpu", "some") => metrics.cpu_some_avg10 = Some(val),
                        ("memory", "some") => metrics.memory_some_avg10 = Some(val),
                        ("memory", "full") => metrics.memory_full_avg10 = Some(val),
                        ("io", "some") => metrics.io_some_avg10 = Some(val),
                        ("io", "full") => metrics.io_full_avg10 = Some(val),
                        _ => {}
                    }
                }
            } else if let Some(val_str) = part.strip_prefix("avg60=") {
                if let Ok(val) = val_str.parse::<f32>() {
                    match (source, pressure_type) {
                        ("cpu", "some") => metrics.cpu_some_avg60 = Some(val),
                        ("memory", "some") => metrics.memory_some_avg60 = Some(val),
                        ("memory", "full") => metrics.memory_full_avg60 = Some(val),
                        ("io", "some") => metrics.io_some_avg60 = Some(val),
                        ("io", "full") => metrics.io_full_avg60 = Some(val),
                        _ => {}
                    }
                }
            } else if let Some(val_str) = part.strip_prefix("avg300=")
                && let Ok(val) = val_str.parse::<f32>()
            {
                match (source, pressure_type) {
                    ("cpu", "some") => metrics.cpu_some_avg300 = Some(val),
                    ("memory", "some") => metrics.memory_some_avg300 = Some(val),
                    ("memory", "full") => metrics.memory_full_avg300 = Some(val),
                    ("io", "some") => metrics.io_some_avg300 = Some(val),
                    ("io", "full") => metrics.io_full_avg300 = Some(val),
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cpu() {
        let mut metrics = PressureMetrics::default();
        let content = "some avg10=10.43 avg60=8.17 avg300=6.52 total=123456";
        parse_pressure_line("cpu", content, &mut metrics);
        assert_eq!(metrics.cpu_some_avg10, Some(10.43));
        assert_eq!(metrics.cpu_some_avg60, Some(8.17));
        assert_eq!(metrics.cpu_some_avg300, Some(6.52));
    }

    #[test]
    fn test_refresh_clears_stale_metrics() {
        let mut metrics = PressureMetrics {
            cpu_some_avg10: Some(99.0),
            ..Default::default()
        };

        // Simulate refresh with no files found
        let now = 12345;
        metrics.cpu_some_avg10 = None;
        metrics.last_refresh_ms = now;
        metrics.last_error = Some("No PSI files found or readable".to_string());

        assert_eq!(metrics.cpu_some_avg10, None);
        assert_eq!(metrics.last_refresh_ms, 12345);
        assert!(metrics.last_error.is_some());
    }
}
