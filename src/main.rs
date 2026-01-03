
mod config;
mod config_watch;
mod cpu;
mod domain;
mod fan;
mod fmt;
mod gamemode;
mod gpu;
mod leds;
mod mem;
mod notify;
mod notifications;
mod power;
mod procwatch;
mod profiles;
mod screen;
mod services;
mod state;
mod sysfs;
mod tempzone;
mod thermal;
mod user_config;
mod web;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use crate::{
    config::*,
    cpu::{avg_util, cpu_utils_by_core, CpuStatSample},
    domain::{base_index_from_ratio, mid_freq, Domain},
    fan::Fan,
    fmt::{fmt_c, fmt_hz, fmt_khz},
    gamemode::{get_foreground_package, load_game_list},
    gpu::read_gpu_util_any,
    leds::Leds,
    notify::{ensure_icon_on_disk, post_notification},
    profiles::{select_active_mode_profile, select_base_led},
    power::ChargeProbe,
    procwatch::ProcWatch,
    screen::{detect_screen_probe, raw_screen_on},
    state::SharedState,
    services::disable_thermal_services,
    sysfs::write_str_if_needed,
    tempzone::{zone_with_hysteresis, TempZone},
    thermal::{describe_paths, read_avg_temp_mc, read_soc_temp_mc},
    user_config::{load_or_init, CONFIG_PATH},
};

fn main() {
    println!("mora_perf_daemon starting");

    disable_thermal_services();
    ensure_icon_on_disk();

    let (cpu_paths, gpu_paths, bat_path) = describe_paths();
    println!(
        "THERM: CPU avg zones {} | GPU avg zones {} | BAT {}",
        cpu_paths.len(),
        gpu_paths.len(),
        if bat_path.is_some() { "ok" } else { "missing" }
    );

    let game_list = load_game_list();
    println!("GAME: list {} pkgs", game_list.len());

    let screen_probe = detect_screen_probe();
    if let Some(p) = &screen_probe {
        println!("SCREEN: {:?}", p);
    } else {
        println!("SCREEN: probe not found (assume ON)");
    }

    let charge_probe = ChargeProbe::detect();
    if charge_probe.is_some() {
        println!("CHG: ok");
    } else {
        println!("CHG: probe not found (assume OFF)");
    }

    // ============================================================
    // Extended functionality: config/profiles/notifications/web UI
    // ============================================================
    let cfg_path = PathBuf::from(CONFIG_PATH);
    let cfg = load_or_init(cfg_path.as_path());
    let shared = Arc::new(RwLock::new(SharedState::new(cfg)));
    let leds = Arc::new(Leds::new());

    // Start background workers.
    config_watch::spawn(shared.clone(), cfg_path.clone());
    notifications::spawn(shared.clone(), leds.clone());
    web::spawn(shared.clone(), leds.clone(), cfg_path.clone());

    // Apply initial LED state (Normal profile, screen assumed ON).
    {
        let cfg = { shared.read().unwrap().config.clone() };
        let prof = select_active_mode_profile(&cfg, false);
                let led_sel = select_base_led(&cfg, true, false, false);

        leds.set_fan_desired(led_sel.fan.clone());
        leds.set_external_desired(led_sel.external.clone());

        let (fan_des, fan_last) = leds.get_fan_state();
        let (ext_des, ext_last) = leds.get_external_state();
let mut s = shared.write().unwrap();
        s.info.active_profile = prof.name;
        s.info.led_profile = led_sel.source;
        s.leds.base_external_desired = ext_des;
        s.leds.base_external_last_applied = ext_last;
        s.leds.fan_desired = fan_des;
        s.leds.fan_last_applied = fan_last;
    }

    let mut fan = Fan::new();
    if fan.is_some() {
        let en = sysfs::read_u64(Path::new(FAN_ENABLE)).unwrap_or(0);
        let lv = sysfs::read_u64(Path::new(FAN_LEVEL)).unwrap_or(0);
        println!("FAN: sysfs ok (en={} lvl={})", en, lv);
    } else {
        println!("FAN: sysfs not found (skip)");
    }

    let gpu_busy_percent_path = {
        let p = PathBuf::from(GPU_BUSY_PERCENT);
        if p.exists() {
            println!("GPUUTIL: gpu_busy_percentage");
            Some(p)
        } else {
            println!("GPUUTIL: gpubusy");
            None
        }
    };
    let gpubusy_path = Path::new(GPU_GPUBUSY);

    let base0 = base_index_from_ratio(CPU0_FREQS, 0.62);
    let base2 = base_index_from_ratio(CPU2_FREQS, 0.48);
    let base5 = base_index_from_ratio(CPU5_FREQS, 0.48);
    let base7 = base_index_from_ratio(CPU7_FREQS, 0.35);
    let baseg = base_index_from_ratio(GPU_FREQS, 0.50);

    let now = Instant::now();

    let mut cpu0 = Domain::new(
        "CPU0", CPU0_FREQS, POLICY0_MIN, POLICY0_MAX, base0, false, now,
        UP_UTIL, SPIKE_DELTA2, SPIKE_DELTA4, HIGH_JUMP2, HIGH_JUMP4,
        60, 50, Duration::from_secs(6), Duration::from_secs(3)
    );
    let mut cpu2 = Domain::new(
        "CPU2", CPU2_FREQS, POLICY2_MIN, POLICY2_MAX, base2, false, now,
        UP_UTIL, SPIKE_DELTA2, SPIKE_DELTA4, HIGH_JUMP2, HIGH_JUMP4,
        60, 50, Duration::from_secs(6), Duration::from_secs(3)
    );
    let mut cpu5 = Domain::new(
        "CPU5", CPU5_FREQS, POLICY5_MIN, POLICY5_MAX, base5, false, now,
        UP_UTIL, SPIKE_DELTA2, SPIKE_DELTA4, HIGH_JUMP2, HIGH_JUMP4,
        60, 50, Duration::from_secs(6), Duration::from_secs(3)
    );
    let mut cpu7 = Domain::new(
        "CPU7", CPU7_FREQS, POLICY7_MIN, POLICY7_MAX, base7, false, now,
        UP_UTIL, SPIKE_DELTA2, SPIKE_DELTA4, HIGH_JUMP2, HIGH_JUMP4,
        60, 50, Duration::from_secs(7), Duration::from_secs(4)
    );
    let mut gpu = Domain::new(
        "GPU", GPU_FREQS, GPU_MIN, GPU_MAX, baseg, true, now,
        UP_UTIL, SPIKE_DELTA2, SPIKE_DELTA4, HIGH_JUMP2, HIGH_JUMP4,
        60, 50, Duration::from_secs(5), Duration::from_secs(3)
    );

    println!(
        "BASE: CPU0 {} | CPU2 {} | CPU5 {} | CPU7 {} | GPU {}",
        fmt_khz(cpu0.freqs[cpu0.base_index]),
        fmt_khz(cpu2.freqs[cpu2.base_index]),
        fmt_khz(cpu5.freqs[cpu5.base_index]),
        fmt_khz(cpu7.freqs[cpu7.base_index]),
        fmt_hz(gpu.freqs[gpu.base_index]),
    );

    // Normal mins
    let cpu0_min_normal = CPU0_FREQS[0];
    let cpu2_min_normal = CPU2_FREQS[0];
    let cpu5_min_normal = CPU5_FREQS[0];
    let cpu7_min_normal = CPU7_FREQS[0];
    let gpu_min_normal = GPU_FREQS[0];

    // Game mins = mid frequency (~50%)
    let cpu0_min_game = mid_freq(CPU0_FREQS);
    let cpu2_min_game = mid_freq(CPU2_FREQS);
    let cpu5_min_game = mid_freq(CPU5_FREQS);
    let cpu7_min_game = mid_freq(CPU7_FREQS);
    let gpu_min_game = mid_freq(GPU_FREQS);

    let cluster0 = [0usize, 1usize];
    let cluster2 = [2usize, 3usize, 4usize];
    let cluster5 = [5usize, 6usize];
    let cluster7 = [7usize];

    let mut cache_u64: HashMap<PathBuf, u64> = HashMap::new();
    let mut cache_str: HashMap<PathBuf, String> = HashMap::new();
    let policy7_gov_path = PathBuf::from(POLICY7_GOV);

    let mut prev_cpu: Option<CpuStatSample> = None;
    let mut last_zone = TempZone::Cool;

    let mut last_enforce = Instant::now();
    let enforce_every_active = Duration::from_secs(ENFORCE_ACTIVE);
    let enforce_every_idle = Duration::from_secs(ENFORCE_IDLE);

    let mut off_streak: u8 = 0;
    let mut screen_on_state = true;
    let mut screen_off_since: Option<Instant> = None;

    let mut charging = false;
    let mut last_chg_check = Instant::now();
    let chg_check_every = Duration::from_secs(CHG_CHECK_EVERY);

    let mut game_mode = false;
    let mut last_game_check = Instant::now();
    let game_check_every = Duration::from_secs(GAME_CHECK_EVERY);
    let mut last_game_pkg: Option<String> = None;

    let mut idle_mode = false;
    let mut idle_accum = Duration::ZERO;

    let mut proc_watch = ProcWatch::new();
    let mut last_proc_check = Instant::now();
    let proc_check_active = Duration::from_secs(3);
    let proc_check_idle = Duration::from_secs(6);

    let mut suspicious: HashMap<String, u8> = HashMap::new();
    let long_off_threshold = Duration::from_secs(LONG_OFF_NOTIFY_SECS);

    let mut last_loop = Instant::now();
    let mut stable_for = Duration::ZERO;

    // Cache config and only clone when it changes (config_watch bumps config_rev).
    let mut cfg_cache = { shared.read().unwrap().config.clone() };
    let mut cfg_rev_cache = { shared.read().unwrap().config_rev };

    loop {
        let now = Instant::now();
        let dt = now.duration_since(last_loop);
        last_loop = now;

        // charging
        if now.duration_since(last_chg_check) >= chg_check_every {
            let new_chg = charge_probe.as_ref().map(|p| p.is_charging()).unwrap_or(false);
            if new_chg != charging {
                charging = new_chg;
                println!("CHG: {}", if charging { "ON" } else { "OFF" });

                if charging {
                    post_notification("Charger connected: charging fan policy enabled");
                } else {
                    post_notification("Charger disconnected: normal fan policy enabled");
                }
            }
            last_chg_check = now;
        }

        // screen
        let screen_on = if let Some(p) = &screen_probe {
            let on = raw_screen_on(p);
            if on {
                off_streak = 0;
                true
            } else {
                off_streak = off_streak.saturating_add(1);
                off_streak < 2
            }
        } else {
            true
        };

        // Game mode detect (only when screen ON)
        if screen_on && now.duration_since(last_game_check) >= game_check_every {
            last_game_check = now;

            let pkg = get_foreground_package();
            let is_game = pkg.as_ref().map(|p| game_list.contains(p)).unwrap_or(false);

            if pkg != last_game_pkg {
                last_game_pkg = pkg.clone();
            }

            if is_game != game_mode {
                game_mode = is_game;

                if game_mode {
                    let name = pkg.clone().unwrap_or_else(|| "?".to_string());
                    println!("GAME: ON ({})", name);
                    post_notification(&format!("Game mode ON: {}", name));
                    // fan baseline now
                    if let Some(f) = fan.as_mut() {
                        f.force_level(&mut cache_u64, GAME_FAN_BASE);
                    }

                    // mins to ~50%
                    cpu0.min_freq = cpu0_min_game;
                    cpu2.min_freq = cpu2_min_game;
                    cpu5.min_freq = cpu5_min_game;
                    cpu7.min_freq = cpu7_min_game;
                    gpu.min_freq = gpu_min_game;

                    // policy7 governor
                    let _ = write_str_if_needed(&policy7_gov_path, GOV_GAME, &mut cache_str, true);
                } else {
                    println!("GAME: OFF");
                    post_notification("Game mode OFF");

                    cpu0.min_freq = cpu0_min_normal;
                    cpu2.min_freq = cpu2_min_normal;
                    cpu5.min_freq = cpu5_min_normal;
                    cpu7.min_freq = cpu7_min_normal;
                    gpu.min_freq = gpu_min_normal;

                    let _ = write_str_if_needed(&policy7_gov_path, GOV_NORMAL, &mut cache_str, true);
                }
            }
        }

        // screen transition -> suspicious notify
        if screen_on != screen_on_state {
            screen_on_state = screen_on;
            if !screen_on {
                screen_off_since = Some(now);
                suspicious.clear();
            } else if let Some(since) = screen_off_since.take() {
                let off_dur = now.duration_since(since);
                if off_dur >= long_off_threshold && !suspicious.is_empty() {
                    let mut v: Vec<(String, u8)> = suspicious.iter().map(|(k, &p)| (k.clone(), p)).collect();
                    v.sort_by(|a, b| b.1.cmp(&a.1));
                    v.truncate(3);

                    let mut msg = String::from("Suspicious background processes: ");
                    for (i, (name, pct)) in v.iter().enumerate() {
                        if i > 0 { msg.push_str(", "); }
                        msg.push_str(&format!("{} {}%", name, pct));
                    }
                    post_notification(&msg);
                }
            }
        }

        // temps (AVG) and choose higher for control
        let cpu_avg_mc = read_avg_temp_mc(&cpu_paths);
        let gpu_avg_mc = read_avg_temp_mc(&gpu_paths);
        let soc_temp_mc = read_soc_temp_mc(cpu_avg_mc, gpu_avg_mc);

        let batt_temp_mc = bat_path.as_ref().and_then(|p| sysfs::read_i32(p));

        let zone = if let Some(t) = soc_temp_mc {
            zone_with_hysteresis(t, last_zone)
        } else {
            last_zone
        };

        if zone != last_zone {
            let c = cpu_avg_mc.map(fmt_c).unwrap_or_else(|| "?".to_string());
            let g = gpu_avg_mc.map(fmt_c).unwrap_or_else(|| "?".to_string());
            let u = soc_temp_mc.map(fmt_c).unwrap_or_else(|| "?".to_string());
            println!("TEMP: cpu {} | gpu {} | use {} -> {:?} (reduce {}%)", c, g, u, zone, zone.reduction_percent());
            last_zone = zone;
        }

        // GPU util
        let ug = read_gpu_util_any(gpu_busy_percent_path.as_deref(), gpubusy_path);

        // CPU util
        let cpu_utils = cpu_utils_by_core(&mut prev_cpu).unwrap_or_default();
        let u0 = avg_util(&cpu_utils, &cluster0);
        let u2 = avg_util(&cpu_utils, &cluster2);
        let u5 = avg_util(&cpu_utils, &cluster5);
        let u7 = avg_util(&cpu_utils, &cluster7);
        let max_cpu_cluster = u0.max(u2).max(u5).max(u7);

        // bg scan (screen OFF)
        let mut bg_over = false;
        if !screen_on {
            let every = if idle_mode { proc_check_idle } else { proc_check_active };
            if now.duration_since(last_proc_check) >= every {
                if let Some((_pid, comm, pct)) = proc_watch.scan_top() {
                    if pct >= BG_CPU_THRESHOLD_PCT {
                        bg_over = true;
                        let entry = suspicious.entry(comm).or_insert(0);
                        *entry = (*entry).max(pct);
                    }
                }
                last_proc_check = now;
            }
        }

        // Read config only when it changed.
        {
            let s = shared.read().unwrap();
            if s.config_rev != cfg_rev_cache {
                cfg_cache = s.config.clone();
                cfg_rev_cache = s.config_rev;
            }
        }
        let cfg = &cfg_cache;

        // charging config switch (defaults to ON)
        let charging_enabled = cfg.charging.enabled;
        let charging_effective = charging && charging_enabled;

        // ------------------------------
        // Profile/LED selection
        // ------------------------------
        let active_prof = select_active_mode_profile(cfg, game_mode);
                let led_sel = select_base_led(cfg, screen_on, charging_effective, game_mode);
        leds.set_fan_desired(led_sel.fan.clone());
        leds.set_external_desired(led_sel.external.clone());

        let (fan_des, fan_last) = leds.get_fan_state();
        let (ext_des, ext_last) = leds.get_external_state();
// fan
        if let Some(f) = fan.as_mut() {
            let soc = soc_temp_mc.unwrap_or(-1);
            f.apply(&mut cache_u64, soc, batt_temp_mc, screen_on, charging_effective, game_mode);
        }

        // idle mode
        let idle_cond = !screen_on && !bg_over && max_cpu_cluster < IDLE_CPU_MAX && ug < IDLE_GPU_MAX;

        if idle_cond { idle_accum += dt; } else { idle_accum = Duration::ZERO; }

        if !idle_mode && idle_accum >= Duration::from_secs(IDLE_ENTER_SECS) {
            idle_mode = true;
            println!("IDLE: enter");

            if !charging_effective && !game_mode {
                cpu0.idx = cpu0.base_index;
                cpu2.idx = cpu2.base_index;
                cpu5.idx = cpu5.base_index;
                cpu7.idx = cpu7.base_index;
                gpu.idx = gpu.base_index;
            }
        }

        if idle_mode && !idle_cond {
            idle_mode = false;
            println!("IDLE: exit");
        }

        // enforce
        let enforce_every = if idle_mode { enforce_every_idle } else { enforce_every_active };
        let force_check = now.duration_since(last_enforce) >= enforce_every;
        if force_check { last_enforce = now; }

        // desired idx update
        let mut any_step = false;
        any_step |= cpu0.desired_step_update(u0, now, dt);
        any_step |= cpu2.desired_step_update(u2, now, dt);
        any_step |= cpu5.desired_step_update(u5, now, dt);
        any_step |= cpu7.desired_step_update(u7, now, dt);
        any_step |= gpu.desired_step_update(ug, now, dt);

        // apply caps
        let mut any_write = false;
        any_write |= cpu0.apply(zone, &mut cache_u64, force_check).unwrap_or(false);
        any_write |= cpu2.apply(zone, &mut cache_u64, force_check).unwrap_or(false);
        any_write |= cpu5.apply(zone, &mut cache_u64, force_check).unwrap_or(false);
        any_write |= cpu7.apply(zone, &mut cache_u64, force_check).unwrap_or(false);
        any_write |= gpu.apply(zone, &mut cache_u64, force_check).unwrap_or(false);

        // STAT
        if force_check {
            let c = cpu_avg_mc.map(|v| v as f32 / 1000.0);
            let g = gpu_avg_mc.map(|v| v as f32 / 1000.0);
            let u = soc_temp_mc.map(|v| v as f32 / 1000.0);
            let b = batt_temp_mc.map(|v| v as f32 / 1000.0);

            let c = c.map(|x| format!("{:.1}C", x)).unwrap_or_else(|| "?".to_string());
            let g = g.map(|x| format!("{:.1}C", x)).unwrap_or_else(|| "?".to_string());
            let u = u.map(|x| format!("{:.1}C", x)).unwrap_or_else(|| "?".to_string());
            let b = b.map(|x| format!("{:.1}C", x)).unwrap_or_else(|| "?".to_string());

            println!(
                "STAT: cpu {} | gpu {} | use {} | bat {} | CPU[{} {} {} {}]% | GPU {}% | scr {} | chg {}{}{}",
                c, g, u, b,
                u0, u2, u5, u7,
                ug,
                if screen_on { "ON" } else { "OFF" },
                if charging { "ON" } else { "OFF" },
                if idle_mode { " | idle" } else { "" },
                if game_mode { " | game" } else { "" },
            );

            {
                let mut st = shared.write().unwrap();
                st.info.cpu_avg_mc = cpu_avg_mc;
                st.info.gpu_avg_mc = gpu_avg_mc;
                st.info.soc_mc = soc_temp_mc;
                st.info.batt_mc = batt_temp_mc;
                st.info.temp_zone = format!("{:?}", zone);
                // mora's reduction percent is returned as u32, UI stores it as u8.
                // Clamp to avoid accidental overflow if implementation changes.
                let rp = zone.reduction_percent();
                st.info.reduce_percent = rp.min(u8::MAX as u32) as u8;
            }
        }

        // Fast status update for other threads/UI.
        {
            let mut st = shared.write().unwrap();
            st.info.screen_on = screen_on;
            st.info.charging = charging;
            st.info.charging_enabled = charging_enabled;
            st.info.charging_effective = charging_effective;
            st.info.game_mode = game_mode;
            st.info.idle_mode = idle_mode;

            // Profiles / LED state (updated continuously)
            st.info.active_profile = active_prof.name.clone();
            st.info.led_profile = led_sel.source.clone();
            st.leds.base_external_desired = ext_des.clone();
            st.leds.base_external_last_applied = ext_last.clone();
            st.leds.fan_desired = fan_des;
            st.leds.fan_last_applied = fan_last;
        }

        // sleep selection
        if any_write || any_step { stable_for = Duration::ZERO; } else { stable_for += dt; }

        let sleep_ms = match zone {
            TempZone::Z120 | TempZone::Z130 => 450,
            _ => {
                if idle_mode {
                    6500
                } else if any_write || any_step {
                    750
                } else if stable_for >= Duration::from_secs(30) {
                    3000
                } else {
                    1500
                }
            }
        };

        std::thread::sleep(Duration::from_millis(sleep_ms));
    }
}
