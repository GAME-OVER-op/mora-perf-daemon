
use std::path::{Path, PathBuf};

use crate::sysfs;

#[derive(Debug)]
pub struct ChargeProbe {
    online_paths: Vec<PathBuf>,
    battery_status_path: Option<PathBuf>,
}

impl ChargeProbe {
    pub fn detect() -> Option<Self> {
        let base = Path::new("/sys/class/power_supply");
        let mut online_paths = Vec::new();
        let mut battery_status_path = None;

        let entries = std::fs::read_dir(base).ok()?;
        for e in entries.flatten() {
            let p = e.path();
            if !p.is_dir() {
                continue;
            }

            let ty = p.join("type");
            let ty_s = std::fs::read_to_string(&ty).unwrap_or_default().trim().to_string();
            let online = p.join("online");
            let status = p.join("status");

            if ty_s.eq_ignore_ascii_case("Battery") {
                if status.exists() && battery_status_path.is_none() {
                    battery_status_path = Some(status);
                }
                continue;
            }

            if online.exists() {
                online_paths.push(online);
            }
        }

        if online_paths.is_empty() && battery_status_path.is_none() {
            None
        } else {
            Some(Self { online_paths, battery_status_path })
        }
    }

    pub fn is_charging(&self) -> bool {
        for p in self.online_paths.iter() {
            if sysfs::read_u64(p).unwrap_or(0) == 1 {
                return true;
            }
        }
        if let Some(st) = &self.battery_status_path {
            if let Some(s) = sysfs::read_to_string(st) {
                let s = s.trim();
                if s.eq_ignore_ascii_case("Charging") || s.eq_ignore_ascii_case("Full") {
                    return true;
                }
            }
        }
        false
    }
}
