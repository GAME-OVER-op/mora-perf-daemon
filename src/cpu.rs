
use std::time::Instant;

#[derive(Clone)]
pub struct CpuStatSample {
    per_cpu: Vec<(u64, u64)>, // (idle_all, total)
    pub t: Instant,
}

fn parse_proc_stat_percpu() -> Option<Vec<(u64, u64)>> {
    let s = std::fs::read_to_string("/proc/stat").ok()?;
    let mut out: Vec<(u64, u64)> = Vec::new();

    for line in s.lines() {
        if !line.starts_with("cpu") { continue; }
        if line.starts_with("cpu ") { continue; }

        let mut it = line.split_whitespace();
        let cpu_label = it.next()?;
        let idx: usize = cpu_label.get(3..)?.parse().ok()?;

        let mut vals: Vec<u64> = Vec::with_capacity(10);
        for v in it {
            if let Ok(x) = v.parse::<u64>() { vals.push(x); }
            else { break; }
        }
        if vals.len() < 4 { continue; }

        let user = vals[0];
        let nice = vals[1];
        let system = vals[2];
        let idle = vals[3];
        let iowait = *vals.get(4).unwrap_or(&0);
        let irq = *vals.get(5).unwrap_or(&0);
        let softirq = *vals.get(6).unwrap_or(&0);
        let steal = *vals.get(7).unwrap_or(&0);

        let idle_all = idle + iowait;
        let total = user + nice + system + idle + iowait + irq + softirq + steal;

        if out.len() <= idx { out.resize(idx + 1, (0, 0)); }
        out[idx] = (idle_all, total);
    }

    if out.is_empty() { None } else { Some(out) }
}

pub fn cpu_utils_by_core(prev: &mut Option<CpuStatSample>) -> Option<Vec<u8>> {
    let now = Instant::now();
    let cur = parse_proc_stat_percpu()?;

    let util = if let Some(p) = prev {
        let mut out = Vec::with_capacity(cur.len());
        for (i, (idle, total)) in cur.iter().enumerate() {
            let (pidle, ptotal) = p.per_cpu.get(i).copied().unwrap_or((0, 0));
            let didle = idle.saturating_sub(pidle);
            let dtotal = total.saturating_sub(ptotal);
            let u = if dtotal == 0 {
                0
            } else {
                let busy = dtotal.saturating_sub(didle);
                (busy.saturating_mul(100) / dtotal).min(100) as u8
            };
            out.push(u);
        }
        out
    } else {
        vec![0u8; cur.len()]
    };

    *prev = Some(CpuStatSample { per_cpu: cur, t: now });
    Some(util)
}

pub fn avg_util(utils: &[u8], cpus: &[usize]) -> u8 {
    if cpus.is_empty() { return 0; }
    let mut sum = 0u32;
    let mut n = 0u32;
    for &c in cpus {
        if let Some(&u) = utils.get(c) {
            sum += u as u32;
            n += 1;
        }
    }
    if n == 0 { 0 } else { (sum / n) as u8 }
}
