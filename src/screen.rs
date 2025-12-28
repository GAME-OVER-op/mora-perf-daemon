
use std::path::PathBuf;

use crate::sysfs;

#[derive(Debug)]
pub enum ScreenProbe {
    FbBlank(PathBuf),
    BacklightBright(PathBuf),
    BacklightPower(PathBuf),
}

pub fn detect_screen_probe() -> Option<ScreenProbe> {
    let fb_blank = PathBuf::from("/sys/class/graphics/fb0/blank");
    if fb_blank.exists() {
        return Some(ScreenProbe::FbBlank(fb_blank));
    }

    let bl_dir = std::path::Path::new("/sys/class/backlight");
    if let Ok(entries) = std::fs::read_dir(bl_dir) {
        for e in entries.flatten() {
            let p = e.path();
            let bright = p.join("brightness");
            if bright.exists() {
                return Some(ScreenProbe::BacklightBright(bright));
            }
            let blp = p.join("bl_power");
            if blp.exists() {
                return Some(ScreenProbe::BacklightPower(blp));
            }
        }
    }
    None
}

pub fn raw_screen_on(probe: &ScreenProbe) -> bool {
    match probe {
        ScreenProbe::FbBlank(p) => sysfs::read_to_string(p)
            .and_then(|s| s.trim().parse::<i32>().ok())
            .map(|v| v == 0)
            .unwrap_or(true),
        ScreenProbe::BacklightBright(p) => sysfs::read_to_string(p)
            .and_then(|s| s.trim().parse::<i32>().ok())
            .map(|v| v > 0)
            .unwrap_or(true),
        ScreenProbe::BacklightPower(p) => sysfs::read_to_string(p)
            .and_then(|s| s.trim().parse::<i32>().ok())
            .map(|v| v == 0)
            .unwrap_or(true),
    }
}
