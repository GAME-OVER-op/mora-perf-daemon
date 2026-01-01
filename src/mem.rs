use std::fs;

/// Read VmRSS (resident set size) in kB from /proc/self/status.
pub fn read_vmrss_kb() -> Option<u64> {
    let s = fs::read_to_string("/proc/self/status").ok()?;
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let n = rest
                .split_whitespace()
                .next()
                .and_then(|x| x.parse::<u64>().ok());
            return n;
        }
    }
    None
}
