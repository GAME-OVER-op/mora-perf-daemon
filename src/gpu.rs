
use std::path::Path;

use crate::sysfs;

fn parse_u8_token(s: &str) -> Option<u8> {
    let tok = s.split_whitespace().next()?.trim();
    let tok = tok.trim_end_matches('%');
    tok.parse::<u8>().ok()
}

fn read_gpu_util_from_busy_percentage(path: &Path) -> Option<u8> {
    let s = sysfs::read_to_string(path)?;
    let v = parse_u8_token(&s)?;
    Some(v.min(100))
}

fn read_gpu_util_from_gpubusy_ratio(path: &Path) -> Option<u8> {
    let s = sysfs::read_to_string(path)?;
    let mut it = s.split_whitespace();
    let busy: u64 = it.next()?.parse().ok()?;
    let total: u64 = it.next()?.parse().ok()?;
    if total == 0 { return Some(0); }
    Some(((busy.saturating_mul(100) / total).min(100)) as u8)
}

pub fn read_gpu_util_any(busy_percent_path: Option<&Path>, gpubusy_path: &Path) -> u8 {
    if let Some(p) = busy_percent_path {
        if let Some(v) = read_gpu_util_from_busy_percentage(p) {
            if v > 0 { return v; }
            if let Some(v2) = read_gpu_util_from_gpubusy_ratio(gpubusy_path) { return v2; }
            return v;
        }
    }
    read_gpu_util_from_gpubusy_ratio(gpubusy_path).unwrap_or(0)
}


fn parse_gpu_freqs_from_str(s: &str) -> Option<Vec<u64>> {
    let mut vals: Vec<u64> = s
        .split_whitespace()
        .filter_map(|t| t.trim().parse::<u64>().ok())
        .filter(|&v| v >= 1_000_000)
        .collect();
    if vals.is_empty() { return None; }
    vals.sort_unstable();
    vals.dedup();
    Some(vals)
}

pub fn load_gpu_freqs_dynamic(fallback: &'static [u64]) -> (&'static [u64], String) {
    const CANDIDATES: &[&str] = &[
        "/sys/class/kgsl/kgsl-3d0/devfreq/available_frequencies",
        "/sys/class/kgsl/kgsl-3d0/gpu_available_frequencies",
        "/sys/class/devfreq/3d00000.qcom,kgsl-3d0/available_frequencies",
    ];

    for path in CANDIDATES {
        let p = Path::new(path);
        if let Some(s) = sysfs::read_to_string(p) {
            if let Some(vals) = parse_gpu_freqs_from_str(&s) {
                let src = format!("dynamic from {}", path);
                let leaked: &'static [u64] = Box::leak(vals.into_boxed_slice());
                return (leaked, src);
            }
        }
    }

    (fallback, "built-in fallback".to_string())
}
