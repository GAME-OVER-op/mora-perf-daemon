
mod config;
mod config_watch;
mod cpu;
mod domain;
mod fan;
mod fmt;
mod gamemode;
mod games;
mod games_watch;
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
mod split_charge;
mod state;
mod sysfs;
mod tempzone;
mod thermal;
mod triggers;
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
    gamemode::get_foreground_package,
    games::{apply_updatable_driver_apps, load_or_init as load_games_or_init},
    gpu::{load_gpu_freqs_dynamic, read_gpu_util_any},
    leds::Leds,
    notify::{ensure_icon_on_disk, post_notification},
    profiles::{select_active_mode_profile, select_base_led},
    power::ChargeProbe,
    procwatch::ProcWatch,
    screen::{detect_screen_probe, raw_screen_on},
    state::SharedState,
    services::disable_thermal_services,
    split_charge::{DesiredSplitCharge, SplitChargeController},
    sysfs::{write_str_if_needed, write_u64_if_needed},
    tempzone::{zone_with_hysteresis, TempZone},
    thermal::{describe_paths, read_avg_temp_mc, read_control_temp_mc, read_soc_temp_mc},
    triggers::TriggerManager,
    user_config::{load_or_init as load_config_or_init, CONFIG_PATH, GAMES_PATH},
};


fn maybe_post_notification(shared: &Arc<RwLock<SharedState>>, message: &str) {
    // Read live config so toggling daemon_notifications takes effect immediately.
    let enabled = { shared.read().unwrap().config.daemon_notifications };
    if enabled {
        post_notification(message);
    }
}

fn cpu_online_path(cpu: usize) -> PathBuf {
    PathBuf::from(format!("/sys/devices/system/cpu/cpu{}/online", cpu))
}

fn is_cpu_online(cpu: usize) -> bool {
    if cpu == 0 {
        return true;
    }
    let p = cpu_online_path(cpu);
    // Many kernels omit `online` for permanently-online cores; treat missing as online.
    if !p.exists() {
        return true;
    }
    sysfs::read_u64(&p).unwrap_or(1) == 1
}

fn set_cpu_online(cpu: usize, online: bool, cache_u64: &mut HashMap<PathBuf, u64>) {
    let p = cpu_online_path(cpu);
    let target = if online { 1u64 } else { 0u64 };
    let _ = write_u64_if_needed(&p, target, cache_u64, true);
}

fn battery_saver_disable_delay(override_dur: Duration) -> Duration {
    let s = override_dur.as_secs_f64();
    if s <= 10.0 {
        Duration::from_secs(5)
    } else if s >= 20.0 {
        Duration::from_secs(30)
    } else {
        let t = (s - 10.0) / 10.0;
        let ms = 5000.0 + t * 25000.0;
        Duration::from_millis(ms.round() as u64)
    }
}

const SCREEN_OFF_CORE_SAVER_SECS: u64 = 30 * 60;

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

    let games_path = PathBuf::from(GAMES_PATH);
    let (games_rt, games_err) = load_games_or_init(games_path.as_path());
    println!(
        "GAMES: list {} pkgs (driver {})",
        games_rt.file.games.len(),
        games_rt.driver_pkgs.len()
    );
    // Sync Android updatable game driver list once at startup.
    apply_updatable_driver_apps(&games_rt.driver_string);

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
    let cfg = load_config_or_init(cfg_path.as_path());
    let shared = Arc::new(RwLock::new(SharedState::new(cfg, games_rt)));
    let leds = Arc::new(Leds::new());

    // Record initial games load status.
    {
        let mut s = shared.write().unwrap();
        s.games_rev = s.games_rev.wrapping_add(1);
        s.last_games_error = games_err;
    }

    // Start background workers.
    config_watch::spawn(shared.clone(), cfg_path.clone());
    games_watch::spawn(shared.clone(), games_path.clone());
    notifications::spawn(shared.clone(), leds.clone());
    web::spawn(shared.clone(), leds.clone(), cfg_path.clone(), games_path.clone());

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

    // Triggers (shoulder buttons -> virtual touch). Optional.
    let triggers = match TriggerManager::init() {
        Ok(t) => {
            println!("TRIG: ready");
            Some(t)
        }
        Err(e) => {
            println!("TRIG: unavailable ({})", e);
            None
        }
    };

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
    let (gpu_freqs, gpu_freqs_source) = load_gpu_freqs_dynamic(GPU_FREQS);
    let baseg = base_index_from_ratio(gpu_freqs, 0.50);

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
        "GPU", gpu_freqs, GPU_MIN, GPU_MAX, baseg, true, now,
        UP_UTIL, SPIKE_DELTA2, SPIKE_DELTA4, HIGH_JUMP2, HIGH_JUMP4,
        60, 50, Duration::from_secs(5), Duration::from_secs(3)
    );

    println!(
        "GPU table: {} ({} steps, {}..{})",
        gpu_freqs_source,
        gpu.freqs.len(),
        fmt_hz(gpu.freqs[0]),
        fmt_hz(*gpu.freqs.last().unwrap()),
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
    let gpu_min_normal = gpu.freqs[0];

    // Game mins = mid frequency (~50%)
    let cpu0_min_game = mid_freq(CPU0_FREQS);
    let cpu2_min_game = mid_freq(CPU2_FREQS);
    let cpu5_min_game = mid_freq(CPU5_FREQS);
    let cpu7_min_game = mid_freq(CPU7_FREQS);
    let gpu_min_game = mid_freq(gpu.freqs);

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

    // Adaptive charging probe interval based on battery percent.
    // - > 80%  -> 15s
    // - > 50%  -> 10s
    // - <= 50% -> 5s
    // If battery percent is unknown, use 10s.
    let chg_check_every = |pct: Option<u8>| -> Duration {
        match pct {
            Some(p) if p > 80 => Duration::from_secs(15),
            Some(p) if p > 50 => Duration::from_secs(10),
            Some(_) => Duration::from_secs(5),
            None => Duration::from_secs(10),
        }
    };

    let mut battery_percent: Option<u8> = None;
    let mut last_batt_check = Instant::now();
    // Battery percent is slow-changing; poll rarely to reduce wakeups.
    let batt_check_every = Duration::from_secs(60);

    let mut game_mode = false;
    // Minimum fan level while current foreground game is active (2..=5). Default matches config::GAME_FAN_BASE.
    let mut game_fan_min_level: u8 = GAME_FAN_BASE;
    // Per-game GPU turbo flag for current foreground game.
    let mut game_gpu_turbo: bool = false;
    // Per-game thermal limit bypass flag.
    let mut game_disable_thermal_limit: bool = false;
    let mut last_triggers_cfg: Option<crate::triggers::ActiveConfig> = None;
    let mut last_game_check = Instant::now();
    let game_check_every = Duration::from_secs(GAME_CHECK_EVERY);
    let mut last_game_pkg: Option<String> = None;
    let mut game_split_charge_cfg = crate::games::SplitChargeConfig::default();
    let mut split_charge = SplitChargeController::new();
    let mut fan_disabled_by_config = false;

    let mut idle_mode = false;
    let mut idle_accum = Duration::ZERO;

    let mut proc_watch = ProcWatch::new();
    let mut last_proc_check = Instant::now();
    // Background proc scan is expensive; do it rarely.
    let proc_check_active = Duration::from_secs(10);
    let proc_check_idle = Duration::from_secs(10);

    let mut suspicious: HashMap<String, u8> = HashMap::new();
    let long_off_threshold = Duration::from_secs(LONG_OFF_NOTIFY_SECS);

    let mut last_loop = Instant::now();
    let mut stable_for = Duration::ZERO;

    // Cache config and only clone when it changes (config_watch bumps config_rev).
    let mut cfg_cache = { shared.read().unwrap().config.clone() };
    let mut cfg_rev_cache = { shared.read().unwrap().config_rev };

    // Smart battery saver runtime state.
    let mut bs_high_streak: Duration = Duration::ZERO;
    let mut bs_override = false;
    let mut bs_override_since: Option<Instant> = None;
    let mut bs_reapply_at: Option<Instant> = None;
    let mut bs_disabled_cores: Vec<usize> = Vec::new();

    loop {
        let now = Instant::now();
        let dt = now.duration_since(last_loop);
        last_loop = now;

        // charging (adaptive interval based on battery percent)
        if now.duration_since(last_chg_check) >= chg_check_every(battery_percent) {
            let new_chg = charge_probe.as_ref().map(|p| p.is_charging()).unwrap_or(false);
            if new_chg != charging {
                charging = new_chg;
                println!("CHG: {}", if charging { "ON" } else { "OFF" });

                if charging {
                    maybe_post_notification(&shared, "Charger connected: charging fan policy enabled");
                } else {
                    maybe_post_notification(&shared, "Charger disconnected: normal fan policy enabled");
                }
            }
            last_chg_check = now;
        }

        // battery percent
        if now.duration_since(last_batt_check) >= batt_check_every {
            battery_percent = charge_probe.as_ref().and_then(|p| p.battery_percent());
            last_batt_check = now;
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

        // Triggers must work only when screen is ON.
        if !screen_on {
            if let Some(mgr) = triggers.as_ref() {
                if last_triggers_cfg.is_some() {
                    mgr.disable();
                    last_triggers_cfg = None;
                    let mut s = shared.write().unwrap();
                    s.info.triggers_active = false;
                    s.info.triggers_left = false;
                    s.info.triggers_right = false;
                    s.info.triggers_pkg = None;
                }
            }
        }

        // Game mode detect (only when screen ON)
        if screen_on && now.duration_since(last_game_check) >= game_check_every {
            last_game_check = now;

            let pkg = get_foreground_package();
            let (is_game, detected_fan_min, detected_gpu_turbo, detected_disable_thermal_limit, detected_split_charge_cfg) = pkg
                .as_deref()
                .map(|p| {
                    let s = shared.read().unwrap();
                    (
                        s.games.is_game(p),
                        s.games.game_fan_min_level(p),
                        s.games.game_gpu_turbo(p),
                        s.games.game_disable_thermal_limit(p),
                        s.games.game_split_charge(p),
                    )
                })
                .unwrap_or((false, GAME_FAN_BASE, false, false, crate::games::SplitChargeConfig::default()));

            // Keep the latest per-game minimum fan level while in game mode.
            if is_game {
                game_fan_min_level = detected_fan_min;
                game_gpu_turbo = detected_gpu_turbo;
                game_disable_thermal_limit = detected_disable_thermal_limit;
                game_split_charge_cfg = detected_split_charge_cfg;
            } else {
                game_fan_min_level = GAME_FAN_BASE;
                game_gpu_turbo = false;
                game_disable_thermal_limit = false;
                game_split_charge_cfg = crate::games::SplitChargeConfig::default();
            }

            // Triggers config (per-game). Active only for foreground game and screen ON.
            if let Some(mgr) = triggers.as_ref() {
                let desired_trig: Option<crate::triggers::ActiveConfig> = pkg
                    .as_deref()
                    .and_then(|p| {
                        let s = shared.read().unwrap();
                        if s.games.is_game(p) {
                            s.games.triggers_for(p)
                        } else {
                            None
                        }
                    })
                    .map(|t| crate::triggers::ActiveConfig {
                        enabled: t.enabled,
                        left: crate::triggers::SideConfig {
                            enabled: t.left.enabled,
                            x_px: t.left.x,
                            y_px: t.left.y,
                        },
                        right: crate::triggers::SideConfig {
                            enabled: t.right.enabled,
                            x_px: t.right.x,
                            y_px: t.right.y,
                        },
                    })
                    .filter(|c| c.enabled && (c.left.enabled || c.right.enabled));

                if desired_trig != last_triggers_cfg {
                    match desired_trig {
                        Some(cfg) => mgr.set_config(cfg),
                        None => mgr.disable(),
                    }
                    last_triggers_cfg = desired_trig;
                }

                let (active, l, r) = match last_triggers_cfg {
                    Some(c) => (true, c.left.enabled, c.right.enabled),
                    None => (false, false, false),
                };
                let mut s = shared.write().unwrap();
                s.info.triggers_active = active;
                s.info.triggers_left = l;
                s.info.triggers_right = r;
                s.info.triggers_pkg = if active { pkg.clone() } else { None };
            }

            if pkg != last_game_pkg {
                last_game_pkg = pkg.clone();

                // If we switched between games, immediately apply that game's minimum.
                if is_game {
                    if let Some(f) = fan.as_mut() {
                        if f.level() < game_fan_min_level {
                            f.force_level(&mut cache_u64, game_fan_min_level);
                        }
                    }
                }
            }

            if is_game != game_mode {
                game_mode = is_game;

                if game_mode {
                    let name = pkg.clone().unwrap_or_else(|| "?".to_string());
                    println!("GAME: ON ({})", name);
                    maybe_post_notification(&shared, &format!("Game mode ON: {}", name));
                    // fan baseline now (per-game minimum)
                    if let Some(f) = fan.as_mut() {
                        if f.level() < game_fan_min_level {
                            f.force_level(&mut cache_u64, game_fan_min_level);
                        }
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
                    maybe_post_notification(&shared, "Game mode OFF");

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
                    maybe_post_notification(&shared, &msg);
                }
            }
        }

        // temps (AVG) and battery temperature for thermal control
        let cpu_avg_mc = read_avg_temp_mc(&cpu_paths);
        let gpu_avg_mc = read_avg_temp_mc(&gpu_paths);
        let batt_temp_mc = bat_path.as_ref().and_then(|p| sysfs::read_i32(p));
        let control_temp_mc = read_control_temp_mc(batt_temp_mc, cpu_avg_mc, gpu_avg_mc);

        let zone = if let Some(t) = control_temp_mc {
            zone_with_hysteresis(t, last_zone)
        } else {
            last_zone
        };

        if zone != last_zone {
            let c = cpu_avg_mc.map(fmt_c).unwrap_or_else(|| "?".to_string());
            let g = gpu_avg_mc.map(fmt_c).unwrap_or_else(|| "?".to_string());
            let b = batt_temp_mc.map(fmt_c).unwrap_or_else(|| "?".to_string());
            let u = control_temp_mc.map(fmt_c).unwrap_or_else(|| "?".to_string());
            println!("TEMP: batt {} | cpu {} | gpu {} | use {} -> {:?} (reduce {}%)", b, c, g, u, zone, zone.reduction_percent());
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
        // Smart battery saver (CPU core hotplug)
        // ------------------------------
        // Average utilization across the base 5 cores (cpu0..cpu4), counting only online CPUs.
        let base_util: u8 = {
            let mut sum: u32 = 0;
            let mut n: u32 = 0;
            for c in 0usize..=4usize {
                if is_cpu_online(c) {
                    if let Some(&u) = cpu_utils.get(c) {
                        sum += u as u32;
                        n += 1;
                    }
                }
            }
            if n == 0 { 0 } else { (sum / n) as u8 }
        };

        let bs_enabled = cfg.battery_saver.enabled;
        let mut offline_by_battery: Vec<usize> = Vec::new();
        let mut offline_by_screen_off: Vec<usize> = Vec::new();

        // Battery thresholds:
        //  - <50%  => disable cpu7
        //  - <35%  => disable cpu5,cpu6,cpu7
        // Overrides:
        //  - charging OR game_mode => always enable all
        //  - if base cores >=90% for 15s => enable all cores
        if bs_enabled && !charging && !game_mode {
            if let Some(pct) = battery_percent {
                if pct < 35 {
                    offline_by_battery.extend([5usize, 6, 7]);
                } else if pct < 50 {
                    offline_by_battery.push(7);
                }
            }
        }

        let battery_baseline_off = !offline_by_battery.is_empty();

        if !bs_enabled || charging || game_mode || !battery_baseline_off {
            // Feature disabled / not applicable => reset battery-saver state.
            bs_high_streak = Duration::ZERO;
            bs_override = false;
            bs_override_since = None;
            bs_reapply_at = None;
            offline_by_battery.clear();
        } else {
            // Override state machine.
            if bs_override {
                if base_util >= 90 {
                    // Still heavy -> cancel pending re-apply.
                    bs_reapply_at = None;
                } else {
                    if bs_reapply_at.is_none() {
                        let since = bs_override_since.unwrap_or(now);
                        let delay = battery_saver_disable_delay(now.duration_since(since));
                        bs_reapply_at = Some(now + delay);
                    }
                    if bs_reapply_at.map(|t| now >= t).unwrap_or(false) {
                        bs_override = false;
                        bs_override_since = None;
                        bs_reapply_at = None;
                        bs_high_streak = Duration::ZERO;
                    }
                }
            }

            if !bs_override {
                if base_util >= 90 {
                    bs_high_streak += dt;
                    if bs_high_streak >= Duration::from_secs(15) {
                        bs_override = true;
                        bs_override_since = Some(now);
                        bs_reapply_at = None;
                        bs_high_streak = Duration::ZERO;
                    }
                } else {
                    bs_high_streak = Duration::ZERO;
                }
            }

            if bs_override {
                offline_by_battery.clear();
            }
        }

        // Additional saver: if the screen stays OFF for 30 minutes, temporarily park cpu5..cpu7.
        // Battery-based saver remains the priority; when the screen turns back ON, only the
        // extra screen-off restriction is removed and the battery policy keeps whatever cores
        // should still stay offline for the current battery %.
        if !charging && !game_mode {
            if let Some(since) = screen_off_since {
                if !screen_on_state && now.duration_since(since) >= Duration::from_secs(SCREEN_OFF_CORE_SAVER_SECS) {
                    offline_by_screen_off.extend([5usize, 6, 7]);
                }
            }
        }

        let mut desired_offline: Vec<usize> = offline_by_battery.clone();
        for c in offline_by_screen_off.iter().copied() {
            if !desired_offline.contains(&c) {
                desired_offline.push(c);
            }
        }
        desired_offline.sort_unstable();

        // Apply desired core states (manage cpu5..cpu7 only).
        for &c in &[5usize, 6, 7] {
            set_cpu_online(c, !desired_offline.contains(&c), &mut cache_u64);
        }

        bs_disabled_cores = offline_by_battery.clone();
        let bs_active = bs_enabled && !charging && !game_mode && !bs_override && !bs_disabled_cores.is_empty();
        let bs_reapply_in = bs_reapply_at.map(|t| if t > now { (t - now).as_secs() } else { 0 });
        let screen_off_saver_active = !offline_by_screen_off.is_empty();

        let split_charge_should_enable = if game_mode
            && game_split_charge_cfg.enabled
            && charging
            && battery_percent.map(|p| p > game_split_charge_cfg.stop_battery_percent).unwrap_or(false)
        {
            true
        } else {
            false
        };
        let desired_split_charge = DesiredSplitCharge {
            should_enable: split_charge_should_enable,
            package: if split_charge_should_enable { last_game_pkg.clone() } else { None },
            stop_battery_percent: game_split_charge_cfg.stop_battery_percent,
        };
        split_charge.sync(desired_split_charge, now);
        let split_charge_status = split_charge.status();

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
            if cfg.use_phone_cooler {
                fan_disabled_by_config = false;
                let soc = read_soc_temp_mc(cpu_avg_mc, gpu_avg_mc).unwrap_or(-1);
                f.apply(
                    &mut cache_u64,
                    soc,
                    batt_temp_mc,
                    screen_on,
                    charging_effective,
                    game_mode,
                    game_fan_min_level,
                );
            } else if !fan_disabled_by_config || f.level() != 0 {
                f.force_level(&mut cache_u64, 0);
                fan_disabled_by_config = true;
            }
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

        
        // GPU turbo: pin min/max to maximum while foreground game requests it.
        let gpu_turbo_active = game_mode && screen_on && game_gpu_turbo;
        if game_mode {
            if gpu_turbo_active {
                gpu.min_freq = gpu.max_freq;
            } else {
                gpu.min_freq = gpu_min_game;
            }
        } else {
            gpu.min_freq = gpu_min_normal;
        }

// desired idx update
        let mut any_step = false;
        any_step |= cpu0.desired_step_update(u0, now, dt);
        any_step |= cpu2.desired_step_update(u2, now, dt);
        any_step |= cpu5.desired_step_update(u5, now, dt);
        any_step |= cpu7.desired_step_update(u7, now, dt);
        if !gpu_turbo_active {
            any_step |= gpu.desired_step_update(ug, now, dt);
        } else {
            // Force GPU domain to max immediately while turbo is active.
            gpu.force_idx(gpu.freqs.len() - 1, now);
        }

        // apply caps
        let effective_zone = if game_mode && game_disable_thermal_limit {
            TempZone::Cool
        } else {
            zone
        };
        let mut any_write = false;
        any_write |= cpu0.apply(effective_zone, &mut cache_u64, force_check).unwrap_or(false);
        any_write |= cpu2.apply(effective_zone, &mut cache_u64, force_check).unwrap_or(false);
        any_write |= cpu5.apply(effective_zone, &mut cache_u64, force_check).unwrap_or(false);
        any_write |= cpu7.apply(effective_zone, &mut cache_u64, force_check).unwrap_or(false);
        any_write |= gpu.apply(effective_zone, &mut cache_u64, force_check).unwrap_or(false);

        // STAT
        if force_check {
            let c = cpu_avg_mc.map(|v| v as f32 / 1000.0);
            let g = gpu_avg_mc.map(|v| v as f32 / 1000.0);
            let u = control_temp_mc.map(|v| v as f32 / 1000.0);
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
                st.info.soc_mc = control_temp_mc;
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

            // Battery saver runtime (updated continuously)
            st.info.battery_percent = battery_percent;
            st.info.battery_saver_active = bs_active;
            st.info.battery_saver_override = bs_override;
            st.info.battery_saver_disabled_cores = bs_disabled_cores.iter().map(|&x| x as u8).collect();
            st.info.battery_saver_reapply_in_sec = bs_reapply_in;
            st.info.screen_off_core_saver_active = screen_off_saver_active;
            st.info.screen_off_core_saver_disabled_cores = offline_by_screen_off.iter().map(|&x| x as u8).collect();
            st.info.split_charge_active = split_charge_status.active;
            st.info.split_charge_package = split_charge_status.package.clone();
            st.info.split_charge_node = split_charge_status.node.clone();
            st.info.split_charge_stop_battery_percent = split_charge_status.target_stop_battery_percent;
            st.info.split_charge_last_error = split_charge_status.last_error.clone();

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
            TempZone::B56 | TempZone::B57 | TempZone::B58 => 450,
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
