use std::{fs, process::{Command, Stdio}};

fn shell(cmd: &str) {
    let _ = Command::new("/system/bin/sh")
        .args(["-c", cmd])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn settings_put_global(key: &str, value: &str) {
    let st = Command::new("settings")
        .args(["put", "global", key, value])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if st.is_err() || !st.as_ref().ok().map(|x| x.success()).unwrap_or(false) {
        shell(&format!("settings put global {} {}", key, value));
    }
}

pub fn set_fan_enable(enable: bool) {
    settings_put_global("nubia_parts_fan_enable", if enable { "1" } else { "0" });
}

pub fn set_fan_speed_level(level: u8) {
    let level = level.clamp(1, 5).to_string();
    settings_put_global("nubia_parts_fan_speed_level", &level);
}

pub fn set_system_triggers_enabled(enable: bool) {
    settings_put_global("nubia_parts_trigger_enable", if enable { "1" } else { "0" });
    let mode = if enable { "1\n" } else { "2\n" };
    for path in [
        "/proc/nubia_key/sar0/mode_operation",
        "/proc/nubia_key/sar1/mode_operation",
    ] {
        if fs::write(path, mode).is_err() {
            shell(&format!("printf '{}' > {}", mode.trim(), path));
        }
    }
    println!("NUBIA_PARTS: triggers {}", if enable { "on" } else { "off" });
}
