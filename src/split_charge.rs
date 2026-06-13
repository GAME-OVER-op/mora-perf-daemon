use std::{fs, io, path::PathBuf, time::{Duration, Instant}};

const SPLIT_CHARGE_RECHECK_EVERY: Duration = Duration::from_secs(90);
const PREFERRED_NODE: &str = "/sys/class/qcom-battery/battery_charging_enabled";
const CANDIDATE_NODES: &[&str] = &[
    PREFERRED_NODE,
    "/sys/class/power_supply/battery/charging_enabled",
    "/sys/class/qcom-battery/charging_enabled",
    "/sys/class/qcom-battery/charge_mode",
    "/sys/module/zte_misc/parameters/charging_enabled",
];

#[derive(Clone, Debug, Default)]
pub struct SplitChargeStatus {
    pub active: bool,
    pub package: Option<String>,
    pub node: Option<String>,
    pub target_stop_battery_percent: Option<u8>,
    pub last_error: Option<String>,
}

#[derive(Debug, Default)]
pub struct SplitChargeController {
    active_node: Option<PathBuf>,
    active_package: Option<String>,
    target_stop_battery_percent: Option<u8>,
    last_recheck: Option<Instant>,
    last_error: Option<String>,
}

impl SplitChargeController {
    pub fn new() -> Self { Self::default() }

    pub fn status(&self) -> SplitChargeStatus {
        SplitChargeStatus {
            active: self.active_node.is_some(),
            package: self.active_package.clone(),
            node: self.active_node.as_ref().map(|p| p.display().to_string()),
            target_stop_battery_percent: self.target_stop_battery_percent,
            last_error: self.last_error.clone(),
        }
    }

    pub fn sync(&mut self, desired: DesiredSplitCharge, now: Instant) {
        if !desired.should_enable {
            if let Err(e) = self.enforce_normal_charge() {
                self.last_error = Some(e);
            }
            return;
        }

        if self.active_node.is_none() {
            if let Err(e) = self.activate(&desired) {
                self.last_error = Some(e);
            }
            self.last_recheck = Some(now);
            return;
        }

        self.active_package = desired.package.clone();
        self.target_stop_battery_percent = Some(desired.stop_battery_percent);

        let need_recheck = self.last_recheck.map(|t| now.duration_since(t) >= SPLIT_CHARGE_RECHECK_EVERY).unwrap_or(true);
        if need_recheck {
            if let Err(e) = self.ensure_disabled() {
                self.last_error = Some(e);
            }
            self.last_recheck = Some(now);
        }
    }

    fn activate(&mut self, desired: &DesiredSplitCharge) -> Result<(), String> {
        let node = detect_writable_node().ok_or_else(|| "No writable charge toggle node found".to_string())?;
        write_zero(&node).map_err(|e| format!("write {} failed: {}", node.display(), e))?;

        self.active_node = Some(node);
        self.active_package = desired.package.clone();
        self.target_stop_battery_percent = Some(desired.stop_battery_percent);
        self.last_error = None;
        Ok(())
    }

    fn ensure_disabled(&mut self) -> Result<(), String> {
        let Some(node) = self.active_node.as_ref() else { return Ok(()); };
        let cur = fs::read_to_string(node)
            .map_err(|e| format!("read {} failed: {}", node.display(), e))?;
        if cur.trim() != "0" {
            write_zero(node).map_err(|e| format!("rewrite {} failed: {}", node.display(), e))?;
        }
        Ok(())
    }

    fn enforce_normal_charge(&mut self) -> Result<(), String> {
        let node = self.active_node.clone().or_else(detect_writable_node);
        if let Some(node) = node {
            let cur = fs::read_to_string(&node)
                .map_err(|e| format!("read {} failed: {}", node.display(), e))?;
            if cur.trim() != "1" {
                write_one(&node).map_err(|e| format!("enable {} failed: {}", node.display(), e))?;
                let after = fs::read_to_string(&node)
                    .map_err(|e| format!("readback {} failed: {}", node.display(), e))?;
                if after.trim() != "1" {
                    write_one(&node).map_err(|e| format!("retry enable {} failed: {}", node.display(), e))?;
                }
            }
        }
        self.active_node = None;
        self.active_package = None;
        self.target_stop_battery_percent = None;
        self.last_recheck = None;
        Ok(())
    }
}

impl Drop for SplitChargeController {
    fn drop(&mut self) {
        let _ = self.enforce_normal_charge();
    }
}

#[derive(Clone, Debug, Default)]
pub struct DesiredSplitCharge {
    pub should_enable: bool,
    pub package: Option<String>,
    pub stop_battery_percent: u8,
}

fn detect_writable_node() -> Option<PathBuf> {
    for cand in CANDIDATE_NODES {
        let p = PathBuf::from(cand);
        if !p.exists() { continue; }
        if fs::OpenOptions::new().write(true).open(&p).is_ok() {
            return Some(p);
        }
    }
    None
}

fn write_zero(path: &PathBuf) -> io::Result<()> {
    fs::write(path, b"0\n")
}

fn write_one(path: &PathBuf) -> io::Result<()> {
    fs::write(path, b"1\n")
}
