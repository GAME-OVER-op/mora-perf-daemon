
use std::{process::Command, thread, time::Duration};

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

    println!("Thermal service disable finished");
}
