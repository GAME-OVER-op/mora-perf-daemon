
use std::{
    collections::HashMap,
    io,
    path::{PathBuf},
    time::{Duration, Instant},
};

use crate::{fmt, sysfs, tempzone::TempZone};

pub fn clamp_to_table(freqs: &[u64], cap: u64) -> usize {
    let mut lo = 0usize;
    let mut hi = freqs.len();
    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        if freqs[mid] <= cap { lo = mid; } else { hi = mid; }
    }
    lo
}

pub fn base_index_from_ratio(freqs: &[u64], ratio: f32) -> usize {
    if freqs.is_empty() { return 0; }
    let n = freqs.len();
    let x = (ratio * (n.saturating_sub(1) as f32)).round() as i32;
    x.clamp(0, (n - 1) as i32) as usize
}

pub fn mid_freq(freqs: &[u64]) -> u64 {
    if freqs.is_empty() { return 0; }
    freqs[freqs.len() / 2]
}

pub struct Domain {
    pub label: &'static str,
    pub freqs: &'static [u64],
    pub min_freq: u64,
    pub max_freq: u64,
    pub min_path: PathBuf,
    pub max_path: PathBuf,

    pub base_index: usize,
    pub idx: usize,

    last_util: u8,
    max_step_up_next_apply: usize,
    hold_until: Instant,
    low_accum: Duration,

    // control params
    pub up_util: u8,
    pub spike_delta2: u8,
    pub spike_delta4: u8,
    pub high_jump2: u8,
    pub high_jump4: u8,

    pub down_util1: u8,
    pub down_util2: u8,
    pub down_after1: Duration,
    pub down_after2: Duration,

    last_applied_idx: usize,
    last_applied_freq: u64,
    pub is_gpu: bool,
}

impl Domain {
    pub fn new(
        label: &'static str,
        freqs: &'static [u64],
        min_path: &str,
        max_path: &str,
        base_index: usize,
        is_gpu: bool,
        now: Instant,
        up_util: u8,
        spike_delta2: u8,
        spike_delta4: u8,
        high_jump2: u8,
        high_jump4: u8,
        down_util1: u8,
        down_util2: u8,
        down_after1: Duration,
        down_after2: Duration,
    ) -> Self {
        let min_freq = freqs[0];
        let max_freq = *freqs.last().unwrap();
        Self {
            label,
            freqs,
            min_freq,
            max_freq,
            min_path: min_path.into(),
            max_path: max_path.into(),
            base_index,
            idx: base_index,
            last_util: 0,
            max_step_up_next_apply: 1,
            hold_until: now,
            low_accum: Duration::ZERO,
            up_util,
            spike_delta2,
            spike_delta4,
            high_jump2,
            high_jump4,
            down_util1,
            down_util2,
            down_after1,
            down_after2,
            last_applied_idx: base_index,
            last_applied_freq: freqs[base_index],
            is_gpu,
        }
    }

    pub fn desired_step_update(&mut self, util: u8, now: Instant, dt: Duration) -> bool {
        let old_idx = self.idx;

        let delta = if util > self.last_util { util - self.last_util } else { 0 };
        self.last_util = util;

        let mut jump_up: usize = 0;
        if util >= self.high_jump4 || delta >= self.spike_delta4 {
            jump_up = 4;
        } else if util >= self.high_jump2 || delta >= self.spike_delta2 {
            jump_up = 2;
        } else if util >= self.up_util {
            jump_up = 1;
        }

        if jump_up > 0 && self.idx + 1 < self.freqs.len() {
            let new_idx = (self.idx + jump_up).min(self.freqs.len() - 1);
            if new_idx != self.idx {
                self.idx = new_idx;
                self.max_step_up_next_apply = jump_up;
                self.hold_until = now + Duration::from_millis(800);
            }
            self.low_accum = Duration::ZERO;
        } else {
            self.max_step_up_next_apply = 1;
        }

        if now >= self.hold_until {
            if util <= self.down_util2 {
                self.low_accum += dt;
                if self.low_accum >= self.down_after2 {
                    self.low_accum = Duration::ZERO;
                    if self.idx > self.base_index {
                        self.idx -= 1;
                    }
                }
            } else if util <= self.down_util1 {
                self.low_accum += dt;
                if self.low_accum >= self.down_after1 {
                    self.low_accum = Duration::ZERO;
                    if self.idx > self.base_index {
                        self.idx -= 1;
                    }
                }
            } else {
                self.low_accum = Duration::ZERO;
            }
        }

        self.idx != old_idx
    }

    fn max_step_down_for_zone(zone: TempZone) -> usize {
        match zone {
            TempZone::Z130 => 3,
            TempZone::Z120 => 2,
            _ => 1,
        }
    }

    pub fn apply(
        &mut self,
        zone: TempZone,
        cache: &mut HashMap<PathBuf, u64>,
        force_check: bool,
    ) -> io::Result<bool> {
        let reduction = zone.reduction_percent();
        let thermal_cap = if reduction == 0 {
            self.max_freq
        } else {
            let keep = 100u64 - reduction as u64;
            (self.max_freq.saturating_mul(keep)) / 100u64
        };

        let desired_freq = self.freqs[self.idx];
        let cap = desired_freq.min(thermal_cap);
        let computed_idx = clamp_to_table(self.freqs, cap);
        let mut target_idx = computed_idx;

        let up_limit = self.max_step_up_next_apply.max(1);
        if target_idx > self.last_applied_idx + up_limit {
            target_idx = self.last_applied_idx + up_limit;
        }
        self.max_step_up_next_apply = 1;

        if target_idx + 1 <= self.last_applied_idx {
            let max_down = Self::max_step_down_for_zone(zone);
            let min_allowed = self.last_applied_idx.saturating_sub(max_down);
            if target_idx < min_allowed {
                target_idx = min_allowed;
            }
        }

        let target_freq = self.freqs[target_idx];

        // IMPORTANT: protect from min > max during throttling / game-min
        let eff_min = self.min_freq.min(target_freq);

        let _ = sysfs::write_u64_if_needed(&self.min_path, eff_min, cache, force_check)?;
        let wrote_max = sysfs::write_u64_if_needed(&self.max_path, target_freq, cache, force_check)?;

        if target_idx != self.last_applied_idx || target_freq != self.last_applied_freq || (force_check && wrote_max) {
            self.last_applied_idx = target_idx;
            self.last_applied_freq = target_freq;

            if self.is_gpu {
                println!("{}: cap {}", self.label, fmt::fmt_hz(target_freq));
            } else {
                println!("{}: cap {}", self.label, fmt::fmt_khz(target_freq));
            }
        }

        Ok(wrote_max)
    }
}
