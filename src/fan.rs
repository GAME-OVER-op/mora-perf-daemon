
use std::{collections::HashMap, path::{Path, PathBuf}};

use crate::{config, sysfs};

pub struct Fan {
    enable_path: PathBuf,
    level_path: PathBuf,
    level: u8, // 0..5
}

impl Fan {
    pub fn new() -> Option<Self> {
        let enable_path = PathBuf::from(config::FAN_ENABLE);
        let level_path = PathBuf::from(config::FAN_LEVEL);
        if enable_path.exists() && level_path.exists() {
            Some(Self { enable_path, level_path, level: 0 })
        } else {
            None
        }
    }

    pub fn sysfs_ok() -> bool {
        Path::new(config::FAN_ENABLE).exists() && Path::new(config::FAN_LEVEL).exists()
    }

    // SoC curve (fan temps -10°C already built into thresholds externally, keep same curve)
    fn level_from_soc_temp(temp_mc: i32) -> u8 {
        if temp_mc < 50_000 { 0 }
        else if temp_mc < 60_000 { 1 }
        else if temp_mc < 70_000 { 2 }
        else if temp_mc < 80_000 { 3 }
        else if temp_mc < 90_000 { 4 }
        else { 5 }
    }

    // Battery curve when charging
    fn level_from_battery_temp(temp_mc: i32) -> u8 {
        if temp_mc < 15_000 { 0 }
        else if temp_mc < 25_000 { 1 }
        else if temp_mc < 30_000 { 2 }
        else if temp_mc < 35_000 { 3 }
        else if temp_mc < 42_000 { 4 }
        else { 5 }
    }

    pub fn force_level(&mut self, cache: &mut HashMap<PathBuf, u64>, level: u8) {
        let lvl = level.min(5);
        self.level = lvl;

        if self.level == 0 {
            let _ = sysfs::write_u64_if_needed(&self.enable_path, 0, cache, true);
            println!("FAN: off");
            return;
        }

        let v = self.level as u64;
        let _ = sysfs::write_u64_if_needed(&self.enable_path, 1, cache, true);
        let _ = sysfs::write_u64_if_needed(&self.level_path, v, cache, true);
        let _ = sysfs::write_u64_if_needed(&self.enable_path, 1, cache, true);
        println!("FAN: {}", self.level);
    }

    pub fn apply(
        &mut self,
        cache: &mut HashMap<PathBuf, u64>,
        soc_temp_mc: i32,
        batt_temp_mc: Option<i32>,
        screen_on: bool,
        charging: bool,
        game_mode: bool,
    ) {
        let soc_level = if soc_temp_mc >= 0 {
            Self::level_from_soc_temp(soc_temp_mc)
        } else {
            0
        };

        let mut target = if charging {
            let batt_level = batt_temp_mc.map(Self::level_from_battery_temp).unwrap_or(0);
            soc_level.max(batt_level)
        } else if screen_on {
            soc_level
        } else {
            0
        };

        // Game mode baseline fan=2, but allow higher from temps/charging
        if game_mode && (screen_on || charging) {
            target = target.max(config::GAME_FAN_BASE);
        }

        // smooth +/-1
        let next = if target > self.level {
            self.level + 1
        } else if target < self.level {
            self.level - 1
        } else {
            self.level
        };

        if next == self.level { return; }
        self.level = next;

        if self.level == 0 {
            let _ = sysfs::write_u64_if_needed(&self.enable_path, 0, cache, true);
            println!("FAN: off");
            return;
        }

        let lvl = self.level as u64;
        let _ = sysfs::write_u64_if_needed(&self.enable_path, 1, cache, true);
        let _ = sysfs::write_u64_if_needed(&self.level_path, lvl, cache, true);
        let _ = sysfs::write_u64_if_needed(&self.enable_path, 1, cache, true);
        println!("FAN: {}", self.level);
    }
}
