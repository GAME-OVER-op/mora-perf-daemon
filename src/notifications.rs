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

fn run_notification_list() -> Result<String, String> {
    let cmd = cmd_bin();
    let su = su_bin();

    // 1) Preferred (tested): shell UID 2000
    let out = Command::new(&su)
        .args(["-lp", "2000", "-c", &format!("{} notification list", cmd)])
        .output();
    if let Ok(out) = out {
        if out.status.success() {
            return Ok(String::from_utf8_lossy(&out.stdout).to_string());
        }
    }

    // 2) Direct call
    let out = Command::new(&cmd)
        .args(["notification", "list"])
        .output()
        .map_err(|e| format!("Не удалось запустить cmd: {e}"))?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).to_string());
    }

    // 3) Plain su
    let out = Command::new(&su)
        .args(["-c", &format!("{} notification list", cmd)])
        .output()
        .map_err(|e| format!("Не удалось запустить su: {e}"))?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).to_string());
    }

    let code = out.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&out.stderr);
    Err(format!("Команда вернула ошибку (code={code}): {stderr}"))
}

fn snapshot() -> Result<HashSet<String>, String> {
    let out = run_notification_list()?;
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
        let mut prev = match snapshot() {
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

        loop {
            thread::sleep(Duration::from_millis(1000));

            let (cfg, screen_on) = {
                let s = shared.read().unwrap();
                (s.config.clone(), s.info.screen_on)
            };

            if !cfg.notifications.enabled {
                if active {
                    leds.external_stop();
                    active = false;
                    ends_at = None;
                    let mut s = shared.write().unwrap();
                    s.leds.external_active = false;
                    s.leds.external_ends_at = None;
                    s.leds.external_started_at = None;
                }
                // keep snapshot in sync to avoid immediate retrigger when re-enabled
                if let Ok(s) = snapshot() {
                    prev = s;
                }
                continue;
            }

            match snapshot() {
                Ok(cur) => {
                    let mut new_found = false;
                    for line in cur.difference(&prev) {
                        let pkg = pkg_from_line(line).unwrap_or("unknown");
                        println!("NOTIF: NEW notification from {}", pkg);
                        new_found = true;
                    }

                    if new_found {
                        // restart scenario
                        let stop = match cfg.notifications.stop_condition.kind {
                            NotificationsStopKind::UntilScreenOn => ScenarioStop::UntilScreenOn,
                            NotificationsStopKind::ForSeconds => ScenarioStop::ForSeconds,
                        };

                        let now = Instant::now();
                        stop_kind = stop;

                        let n = cfg.notifications.for_seconds.max(1);
                        let end = if screen_on {
                            Some(now + Duration::from_secs(n))
                        } else {
                            match stop_kind {
                                ScenarioStop::UntilScreenOn => None,
                                ScenarioStop::ForSeconds => Some(now + Duration::from_secs(n)),
                            }
                        };

                        let ext = cfg.notifications.external_led.clone();
                        if let Err(e) = leds.external_start(ext.clone()) {
                            eprintln!("LED: external_start error: {}", e);
                        }

                        active = true;
                        ends_at = end;

                        {
                            let mut s = shared.write().unwrap();
                            s.leds.external_active = true;
                            s.leds.external_setting = Some(ext);
                            s.leds.external_stop_kind = cfg.notifications.stop_condition.kind;
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

            // If screen became ON and stop=until_screen_on -> stop.
            if active {
                let (screen_on, stop_kind_cfg) = {
                    let s = shared.read().unwrap();
                    (s.info.screen_on, s.leds.external_stop_kind)
                };
                if screen_on && matches!(stop_kind_cfg, NotificationsStopKind::UntilScreenOn) {
                    leds.external_stop();
                    active = false;
                    ends_at = None;
                    let mut s = shared.write().unwrap();
                    s.leds.external_active = false;
                    s.leds.external_ends_at = None;
                    s.leds.external_started_at = None;
                }
            }
        }
    });
}
