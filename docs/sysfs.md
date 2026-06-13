# Sysfs nodes

## CPU (cpufreq)
- `/sys/devices/system/cpu/cpufreq/policy0/scaling_min_freq`
- `/sys/devices/system/cpu/cpufreq/policy0/scaling_max_freq`
- `/sys/devices/system/cpu/cpufreq/policy2/scaling_min_freq`
- `/sys/devices/system/cpu/cpufreq/policy2/scaling_max_freq`
- `/sys/devices/system/cpu/cpufreq/policy5/scaling_min_freq`
- `/sys/devices/system/cpu/cpufreq/policy5/scaling_max_freq`
- `/sys/devices/system/cpu/cpufreq/policy7/scaling_min_freq`
- `/sys/devices/system/cpu/cpufreq/policy7/scaling_max_freq`

## GPU (KGSL)
- `/sys/class/kgsl/kgsl-3d0/devfreq/min_freq`
- `/sys/class/kgsl/kgsl-3d0/devfreq/max_freq`
- `/sys/class/kgsl/kgsl-3d0/gpu_busy_percentage` (optional)
- `/sys/class/kgsl/kgsl-3d0/gpubusy` (fallback)

## Fan
- `/sys/kernel/fan/fan_enable`
- `/sys/kernel/fan/fan_speed_level`

## Screen state
- `/sys/class/graphics/fb0/blank` OR
- `/sys/class/backlight/*/brightness` OR
- `/sys/class/backlight/*/bl_power`

## Charging state
- `/sys/class/power_supply/*/online`
- `/sys/class/power_supply/*/status`
