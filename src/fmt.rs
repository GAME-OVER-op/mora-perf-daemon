
pub fn fmt_khz(khz: u64) -> String {
    if khz >= 1_000_000 {
        format!("{:.2}GHz", khz as f64 / 1_000_000.0)
    } else {
        format!("{:.0}MHz", khz as f64 / 1_000.0)
    }
}

pub fn fmt_hz(hz: u64) -> String {
    if hz >= 1_000_000_000 {
        format!("{:.2}GHz", hz as f64 / 1_000_000_000.0)
    } else {
        format!("{:.0}MHz", hz as f64 / 1_000_000.0)
    }
}

pub fn fmt_c(mc: i32) -> String {
    format!("{:.1}C", mc as f32 / 1000.0)
}
