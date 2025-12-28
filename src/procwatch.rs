
use std::collections::HashMap;

fn read_total_cpu_jiffies() -> Option<u64> {
    let s = std::fs::read_to_string("/proc/stat").ok()?;
    for line in s.lines() {
        if line.starts_with("cpu ") {
            let mut sum = 0u64;
            for v in line.split_whitespace().skip(1) {
                if let Ok(x) = v.parse::<u64>() {
                    sum = sum.saturating_add(x);
                }
            }
            return Some(sum);
        }
    }
    None
}

fn parse_pid_stat(line: &str) -> Option<(String, u64)> {
    let l = line.find('(')?;
    let r = line.rfind(')')?;
    if r <= l { return None; }

    let comm = line[l + 1..r].to_string();
    let after = &line[r + 1..];

    let mut it = after.split_whitespace();
    let _state = it.next()?; // field 3

    let rest: Vec<&str> = it.collect();
    if rest.len() < 12 { return None; }

    // utime field14, stime field15 -> rest[10] rest[11]
    let utime: u64 = rest[10].parse().ok()?;
    let stime: u64 = rest[11].parse().ok()?;
    Some((comm, utime.saturating_add(stime)))
}

pub struct ProcWatch {
    last_total: u64,
    last_ticks: HashMap<u32, u64>,
}

impl ProcWatch {
    pub fn new() -> Self {
        Self { last_total: 0, last_ticks: HashMap::new() }
    }

    /// Returns top pid, comm, and estimated % of total CPU time since last scan.
    pub fn scan_top(&mut self) -> Option<(u32, String, u8)> {
        let total = read_total_cpu_jiffies()?;
        if self.last_total == 0 {
            self.last_total = total;
            return None;
        }
        let dtotal = total.saturating_sub(self.last_total);
        self.last_total = total;
        if dtotal == 0 { return None; }

        let mut new_map: HashMap<u32, u64> = HashMap::new();
        let mut best: Option<(u32, String, u64)> = None; // pid, comm, dticks

        if let Ok(entries) = std::fs::read_dir("/proc") {
            for e in entries.flatten() {
                let name = match e.file_name().into_string() {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let pid: u32 = match name.parse() {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let stat_path = format!("/proc/{}/stat", pid);
                let line = match std::fs::read_to_string(&stat_path) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let (comm, ticks) = match parse_pid_stat(&line) {
                    Some(v) => v,
                    None => continue,
                };

                let prev = self.last_ticks.get(&pid).copied().unwrap_or(ticks);
                let dt = ticks.saturating_sub(prev);

                new_map.insert(pid, ticks);

                if dt == 0 { continue; }

                let replace = match best.as_ref() {
                    None => true,
                    Some((_bp, _bc, bdt)) => dt > *bdt,
                };
                if replace {
                    best = Some((pid, comm, dt));
                }
            }
        }

        self.last_ticks = new_map;

        let (pid, comm, dt) = best?;
        let pct = ((dt.saturating_mul(100) / dtotal).min(100)) as u8;
        Some((pid, comm, pct))
    }
}
