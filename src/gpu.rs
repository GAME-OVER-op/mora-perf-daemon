
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
