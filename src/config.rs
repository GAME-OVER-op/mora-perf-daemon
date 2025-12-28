
pub const ICON_DST: &str = "/data/local/tmp/mora.png";
pub const ICON_URI: &str = "file:///data/local/tmp/mora.png";

// Thermal zones (AVG)
pub const CPU_ZONE_IDS: &[u32] = &[
    10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 25, 26, 27, 28, 29,
];
pub const GPU_ZONE_IDS: &[u32] = &[41, 42, 43, 44, 45, 46, 47, 48];
pub const BAT_ZONE_ID: u32 = 74;

// Sysfs paths
pub const POLICY0_MIN: &str = "/sys/devices/system/cpu/cpufreq/policy0/scaling_min_freq";
pub const POLICY0_MAX: &str = "/sys/devices/system/cpu/cpufreq/policy0/scaling_max_freq";
pub const POLICY2_MIN: &str = "/sys/devices/system/cpu/cpufreq/policy2/scaling_min_freq";
pub const POLICY2_MAX: &str = "/sys/devices/system/cpu/cpufreq/policy2/scaling_max_freq";
pub const POLICY5_MIN: &str = "/sys/devices/system/cpu/cpufreq/policy5/scaling_min_freq";
pub const POLICY5_MAX: &str = "/sys/devices/system/cpu/cpufreq/policy5/scaling_max_freq";
pub const POLICY7_MIN: &str = "/sys/devices/system/cpu/cpufreq/policy7/scaling_min_freq";
pub const POLICY7_MAX: &str = "/sys/devices/system/cpu/cpufreq/policy7/scaling_max_freq";
pub const POLICY7_GOV: &str = "/sys/devices/system/cpu/cpufreq/policy7/scaling_governor";

pub const GPU_MIN: &str = "/sys/class/kgsl/kgsl-3d0/devfreq/min_freq";
pub const GPU_MAX: &str = "/sys/class/kgsl/kgsl-3d0/devfreq/max_freq";
pub const GPU_BUSY_PERCENT: &str = "/sys/class/kgsl/kgsl-3d0/gpu_busy_percentage";
pub const GPU_GPUBUSY: &str = "/sys/class/kgsl/kgsl-3d0/gpubusy";

pub const FAN_ENABLE: &str = "/sys/kernel/fan/fan_enable";
pub const FAN_LEVEL: &str = "/sys/kernel/fan/fan_speed_level";

// Frequencies (hardcoded)
pub const CPU0_FREQS: &[u64] = &[
    364800, 460800, 556800, 672000, 787200, 902400, 1017600, 1132800, 1248000,
    1344000, 1459200, 1574400, 1689600, 1804800, 1920000, 2035200, 2150400, 2265600,
];
pub const CPU2_FREQS: &[u64] = &[
    499200, 614400, 729600, 844800, 960000, 1075200, 1190400, 1286400, 1401600,
    1497600, 1612800, 1708800, 1824000, 1920000, 2035200, 2131200, 2188800, 2246400,
    2323200, 2380800, 2438400, 2515200, 2572800, 2630400, 2707200, 2764800, 2841600,
    2899200, 2956800, 3014400, 3072000, 3148800,
];
pub const CPU5_FREQS: &[u64] = &[
    499200, 614400, 729600, 844800, 960000, 1075200, 1190400, 1286400, 1401600,
    1497600, 1612800, 1708800, 1824000, 1920000, 2035200, 2131200, 2188800, 2246400,
    2323200, 2380800, 2438400, 2515200, 2572800, 2630400, 2707200, 2764800, 2841600,
    2899200, 2956800,
];
pub const CPU7_FREQS: &[u64] = &[
    480000, 576000, 672000, 787200, 902400, 1017600, 1132800, 1248000, 1363200,
    1478400, 1593600, 1708800, 1824000, 1939200, 2035200, 2112000, 2169600, 2246400,
    2304000, 2380800, 2438400, 2496000, 2553600, 2630400, 2688000, 2745600, 2803200,
    2880000, 2937600, 2995200, 3052800, 3110400, 3187200, 3244800, 3302400,
];
pub const GPU_FREQS: &[u64] = &[
    231000000, 310000000, 366000000, 422000000, 500000000, 578000000, 629000000,
    680000000, 720000000, 770000000, 834000000, 903000000, 916000000,
];

// Loops / timings
pub const ENFORCE_ACTIVE: u64 = 6;
pub const ENFORCE_IDLE: u64 = 18;
pub const CHG_CHECK_EVERY: u64 = 2;
pub const GAME_CHECK_EVERY: u64 = 2;

// Suspicious background proc scan
pub const BG_CPU_THRESHOLD_PCT: u8 = 15;
pub const LONG_OFF_NOTIFY_SECS: u64 = 30;

// Idle condition thresholds
pub const IDLE_ENTER_SECS: u64 = 10;
pub const IDLE_CPU_MAX: u8 = 15;
pub const IDLE_GPU_MAX: u8 = 10;

// Perf control targets
pub const UP_UTIL: u8 = 70;
pub const SPIKE_DELTA2: u8 = 20;
pub const SPIKE_DELTA4: u8 = 35;
pub const HIGH_JUMP2: u8 = 85;
pub const HIGH_JUMP4: u8 = 95;

// Fan: game mode baseline
pub const GAME_FAN_BASE: u8 = 2;

// Game mode governor
pub const GOV_GAME: &str = "performance";
pub const GOV_NORMAL: &str = "walt";
