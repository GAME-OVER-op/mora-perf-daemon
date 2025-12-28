
use std::path::PathBuf;

use crate::{config::{BAT_ZONE_ID, CPU_ZONE_IDS, GPU_ZONE_IDS}, sysfs};

pub fn zone_temp_path(id: u32) -> PathBuf {
    PathBuf::from(format!("/sys/class/thermal/thermal_zone{}/temp", id))
}

pub fn build_paths(ids: &[u32]) -> Vec<PathBuf> {
    let mut v = Vec::new();
    for &id in ids {
        let p = zone_temp_path(id);
        if p.exists() {
            v.push(p);
        }
    }
    v
}

pub fn battery_path() -> Option<PathBuf> {
    let p = zone_temp_path(BAT_ZONE_ID);
    if p.exists() { Some(p) } else { None }
}

pub fn read_avg_temp_mc(paths: &[PathBuf]) -> Option<i32> {
    let mut sum: i64 = 0;
    let mut n: i64 = 0;
    for p in paths {
        if let Some(v) = sysfs::read_i32(p) {
            sum += v as i64;
            n += 1;
        }
    }
    if n == 0 { None } else { Some((sum / n) as i32) }
}

pub fn read_soc_temp_mc(cpu_avg: Option<i32>, gpu_avg: Option<i32>) -> Option<i32> {
    match (cpu_avg, gpu_avg) {
        (Some(c), Some(g)) => Some(c.max(g)),
        (Some(c), None) => Some(c),
        (None, Some(g)) => Some(g),
        (None, None) => None,
    }
}

pub fn describe_paths() -> (Vec<PathBuf>, Vec<PathBuf>, Option<PathBuf>) {
    (build_paths(CPU_ZONE_IDS), build_paths(GPU_ZONE_IDS), battery_path())
}
