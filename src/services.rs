
use std::{fs, process::Command, thread, time::Duration};

use crate::config::{CPU_ZONE_IDS, GPU_ZONE_IDS};

pub fn disable_thermal_services() {
    println!("Disabling thermal services...");

    let stop_services = vec![
        "android.thermal-hal",
        "vendor.thermal-engine",
        "vendor.thermal_manager",
        "vendor.thermal-manager",
        "vendor.thermal-hal-2-0",
        "vendor.thermal-symlinks",
        "thermal_mnt_hal_service",
        "thermal",
        "mi_thermald",
        "thermald",
        "thermalloadalgod",
        "thermalservice",
        "sec-thermal-1-0",
        "debug_pid.sec-thermal-1-0",
        "thermal-engine",
        "vendor.thermal-hal-1-0",
        "vendor-thermal-1-0",
        "thermal-hal",
        "vendor.qti.hardware.perf2-hal-service",
        "qti-msdaemon_vendor-0",
        "qti-msdaemon_vendor-1",
        "qti-ssdaemon_vendor",
    ];

    for service in stop_services {
        let output = Command::new("stop").arg(service).output();
        match output {
            Ok(output) => {
                if output.status.success() {
                    println!("Stopped: {}", service);
                } else {
                    println!("Stop failed {}: {}", service, output.status);
                }
            }
            Err(e) => println!("Stop error {}: {}", service, e),
        }
        thread::sleep(Duration::from_millis(50));
    }

    let setprop_commands = vec![
        ("init.svc.thermal", "stopped"),
        ("init.svc.thermal-managers", "stopped"),
        ("init.svc.thermal_manager", "stopped"),
        ("init.svc.thermal_mnt_hal_service", "stopped"),
        ("init.svc.thermal-engine", "stopped"),
        ("init.svc.mi-thermald", "stopped"),
        ("init.svc.thermalloadalgod", "stopped"),
        ("init.svc.thermalservice", "stopped"),
        ("init.svc.thermal-hal", "stopped"),
        ("init.svc.vendor.thermal-symlinks", ""),
        ("init.svc.android.thermal-hal", "stopped"),
        ("init.svc.vendor.thermal-hal", "stopped"),
        ("init.svc.thermal-manager", "stopped"),
        ("init.svc.vendor-thermal-hal-1-0", "stopped"),
        ("init.svc.vendor.thermal-hal-1-0", "stopped"),
        ("init.svc.vendor.thermal-hal-2-0.mtk", "stopped"),
        ("init.svc.vendor.thermal-hal-2-0", "stopped"),
    ];

    for (prop, value) in setprop_commands {
        let output = Command::new("setprop").arg(prop).arg(value).output();
        match output {
            Ok(output) => {
                if output.status.success() {
                    println!("setprop {}={}", prop, value);
                } else {
                    println!("setprop failed {}={}: {}", prop, value, output.status);
                }
            }
            Err(e) => println!("setprop error {}={}: {}", prop, value, e),
        }
        thread::sleep(Duration::from_millis(50));
    }

    // --- Burn mode additions (port of manual diag script) ---

    // 1) Hard-kill thermal HAL/engine processes that survive `stop`.
    //    e.g. android.hardware.thermal-service.qti lingers with init.svc=stopped
    //    yet keeps applying mitigations through cooling devices.
    let kill_patterns = vec![
        "thermal-service.qti",
        "android.hardware.thermal",
        "thermal-engine",
        "thermald",
    ];
    for pat in kill_patterns {
        match Command::new("pkill").arg("-f").arg(pat).output() {
            Ok(o) => println!("pkill -f {} -> {}", pat, o.status),
            Err(e) => println!("pkill error {}: {}", pat, e),
        }
        thread::sleep(Duration::from_millis(50));
    }

    // 2) Disable in-kernel thermal (step_wise) on CPU/GPU zones so cooling
    //    devices (cpufreq-cpuN / cpu-clusterN / gpu) stop capping frequency.
    //    Battery/BCL zones are intentionally left untouched.
    for &id in CPU_ZONE_IDS.iter().chain(GPU_ZONE_IDS.iter()) {
        let path = format!("/sys/class/thermal/thermal_zone{}/mode", id);
        match fs::write(&path, "disabled") {
            Ok(_) => println!("zone {} mode=disabled", id),
            Err(e) => println!("zone {} disable failed: {}", id, e),
        }
    }

    // 3) Reset any already-engaged CPU/GPU cooling devices back to 0.
    if let Ok(entries) = fs::read_dir("/sys/class/thermal") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("cooling_device") {
                continue;
            }
            let p = entry.path();
            let ty = fs::read_to_string(p.join("type")).unwrap_or_default();
            let ty = ty.trim();
            if ty.starts_with("cpufreq-")
                || ty.starts_with("cpu-cluster")
                || ty.starts_with("thermal-cluster")
                || ty == "gpu"
            {
                let _ = fs::write(p.join("cur_state"), "0");
            }
        }
    }

    // 4) Unbind the userspace LMh driver. The real enforcer lives in CPUCP
    //    firmware (hardware), but we detach what we can from userspace.
    match fs::write(
        "/sys/bus/platform/drivers/msm_lmh_dcvs/unbind",
        "soc:qcom,limits-dcvs",
    ) {
        Ok(_) => println!("lmh: unbound msm_lmh_dcvs"),
        Err(e) => println!("lmh: unbind skipped ({})", e),
    }

    println!("Thermal service disable finished");
}
