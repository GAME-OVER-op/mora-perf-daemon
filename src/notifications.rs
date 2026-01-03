use crate::{leds::Leds, state::SharedState, user_config::NotificationsStopKind};
use std::{
    collections::HashSet,
    fs,
    process::Command,
    sync::{Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

// This module intentionally mirrors `script/main.rs` logic for notifications.
// The tested approach is to run `cmd notification list` as shell UID (2000)
// via `su -lp 2000 ...`.

fn find_bin(candidates: &[&str]) -> Option<String> {
    for p in candidates {
        if fs::metadata(p).is_ok() {
            return Some(p.to_string());
        }
    }
    None
}

fn cmd_bin() -> String {
    find_bin(&["/system/bin/cmd", "/system/xbin/cmd"]).unwrap_or_else(|| "cmd".to_string())
}

fn su_bin() -> String {
    find_bin(&["/system/bin/su", "/system/xbin/su", "/sbin/su", "/su/bin/su"])
        .unwrap_or_else(|| "su".to_string())
}

fn pkg_from_line(line: &str) -> Option<&str> {
    // Expected format like: "21|com.google.android.gm|..."
    line.split('|').nth(1)
}

fn run_notification_list(cmd: &str, su: &str) -> Result<String, String> {

    // 1) Preferred (tested): shell UID 2000
    let out = Command::new(su)
        .args(["-lp", "2000", "-c", &format!("{} notification list", cmd)])
        .output();
    if let Ok(out) = out {
        if out.status.success() {
            return Ok(String::from_utf8_lossy(&out.stdout).to_string());
        }
    }

    // 2) Direct call
    let out = Command::new(cmd)
        .args(["notification", "list"])
        .output()
        .map_err(|e| format!("Failed to run cmd: {e}"))?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).to_string());
    }

    // 3) Plain su
    let out = Command::new(su)
        .args(["-c", &format!("{} notification list", cmd)])
        .output()
        .map_err(|e| format!("Failed to run su: {e}"))?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).to_string());
    }

    let code = out.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&out.stderr);
    Err(format!("Command failed (code={code}): {stderr}"))
}

fn snapshot(cmd: &str, su: &str) -> Result<HashSet<String>, String> {
    let out = run_notification_list(cmd, su)?;
    let mut set = HashSet::new();
    for line in out.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Store raw line (exactly like the tested tool).
        set.insert(line.to_string());
    }
    Ok(set)
}

#[derive(Debug, Clone, Copy)]
enum ScenarioStop {
    UntilScreenOn,
    ForSeconds,
}

/// Notification watcher: any newly appeared notification triggers external LED scenario.
pub fn spawn(shared: Arc<RwLock<SharedState>>, leds: Arc<Leds>) {
    thread::spawn(move || {
        let cmd = cmd_bin();
        let su = su_bin();

        let mut prev = match snapshot(&cmd, &su) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("NOTIF: initial snapshot error: {}", e);
                HashSet::new()
            }
        };

        let mut active = false;
        let mut ends_at: Option<Instant> = None;
        let mut stop_kind = ScenarioStop::UntilScreenOn;

        // throttle error logs
        let mut last_err_at = Instant::now() - Duration::from_secs(3600);
        let mut last_disabled_sync = Instant::now() - Duration::from_secs(3600);

        loop {
            // Read only what we need from shared state/config.
            let (enabled, stop_kind_cfg, for_seconds, ext_setting, screen_on) = {
                let s = shared.read().unwrap();
                let n = &s.config.notifications;
                (
                    n.enabled,
                    n.stop_condition.kind,
                    n.for_seconds.max(1),
                    n.external_led.clone(),
                    s.info.screen_on,
                )
            };

            // If screen became ON and stop=until_screen_on -> stop.
            if active && screen_on && matches!(stop_kind_cfg, NotificationsStopKind::UntilScreenOn) {
                leds.external_stop();
                active = false;
                ends_at = None;
                let mut s = shared.write().unwrap();
                s.leds.external_active = false;
                s.leds.external_ends_at = None;
                s.leds.external_started_at = None;
            }

            if !enabled {
                if active {
                    leds.external_stop();
                    active = false;
                    ends_at = None;
                    let mut s = shared.write().unwrap();
                    s.leds.external_active = false;
                    s.leds.external_ends_at = None;
                    s.leds.external_started_at = None;
                }

                // Keep snapshot in sync to avoid immediate retrigger when re-enabled,
                // but do it rarely to save power.
                if last_disabled_sync.elapsed() > Duration::from_secs(10) {
                    if let Ok(snap) = snapshot(&cmd, &su) {
                        prev = snap;
                    }
                    last_disabled_sync = Instant::now();
                }

                thread::sleep(Duration::from_millis(5000));
                continue;
            }

            match snapshot(&cmd, &su) {
                Ok(cur) => {
                    let mut new_found = false;
                    for line in cur.difference(&prev) {
                        let pkg = pkg_from_line(line).unwrap_or("unknown");
                        println!("NOTIF: NEW notification from {}", pkg);
                        new_found = true;
                    }

                    if new_found {
                        // restart scenario
                        stop_kind = match stop_kind_cfg {
                            NotificationsStopKind::UntilScreenOn => ScenarioStop::UntilScreenOn,
                            NotificationsStopKind::ForSeconds => ScenarioStop::ForSeconds,
                        };

                        let now = Instant::now();
                        let end = if screen_on {
                            Some(now + Duration::from_secs(for_seconds))
                        } else {
                            match stop_kind {
                                ScenarioStop::UntilScreenOn => None,
                                ScenarioStop::ForSeconds => Some(now + Duration::from_secs(for_seconds)),
                            }
                        };

                        if let Err(e) = leds.external_start(ext_setting.clone()) {
                            eprintln!("LED: external_start error: {}", e);
                        }

                        active = true;
                        ends_at = end;
                        {
                            let mut s = shared.write().unwrap();
                            s.leds.external_active = true;
                            s.leds.external_setting = Some(ext_setting.clone());
                            s.leds.external_stop_kind = stop_kind_cfg;
                            s.leds.external_started_at = Some(now);
                            s.leds.external_ends_at = end;
                        }
                    }

                    prev = cur;
                }
                Err(e) => {
                    if last_err_at.elapsed() > Duration::from_secs(30) {
                        eprintln!("NOTIF: snapshot error: {}", e);
                        last_err_at = Instant::now();
                    }
                }
            }

            // stop checks
            if active {
                let now = Instant::now();
                let stop_now = if screen_on {
                    ends_at.map(|t| now >= t).unwrap_or(true)
                } else {
                    match stop_kind {
                        ScenarioStop::UntilScreenOn => false,
                        ScenarioStop::ForSeconds => ends_at.map(|t| now >= t).unwrap_or(false),
                    }
                };

                if stop_now {
                    leds.external_stop();
                    active = false;
                    ends_at = None;
                    let mut s = shared.write().unwrap();
                    s.leds.external_active = false;
                    s.leds.external_ends_at = None;
                    s.leds.external_started_at = None;
                }
            }

            // Poll interval (save power when screen is ON).
            let ms = if screen_on { 3500 } else { 1000 };
            thread::sleep(Duration::from_millis(ms));
        }
    });
}
