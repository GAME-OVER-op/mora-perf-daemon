# mora-perf-daemon

Root performance daemon for **LineageOS on Nubia Red Magic 9 Pro (`tiro`)**:
CPU cpufreq caps, Adreno/KGSL GPU devfreq caps, thermal-based scaling, built-in fan control, idle/sleep mode, and Android notifications.

Target device/ROM reference: ItsVixano LineageOS Wiki — `tiro` (Red Magic 9 Pro family).

## Target device

- Device: Nubia Red Magic 9 Pro (codename `tiro`)
- SoC: Snapdragon 8 Gen 3 (SM8650)
- GPU: Adreno 750
- Kernel: 6.1
- ROM: LineageOS (see wiki)

This project is tuned for the `tiro` family (Red Magic 9 / 9 Pro+ / 9S Pro / 9S Pro+).

## Features

### CPU (cpufreq)
- Controls cpufreq policies via:
  - `scaling_min_freq` (always set to the minimum available)
  - `scaling_max_freq` (dynamic cap)
- Fast ramp-up on spikes (skips steps)
- Slow ramp-down to stay around ~70% utilization

### GPU (KGSL)
- Reads GPU load from:
  - `gpu_busy_percentage` (preferred if present)
  - `gpubusy` (fallback)
- Caps GPU via `devfreq/min_freq` + `devfreq/max_freq`

### Thermal protection
- Uses multiple thermal zones (cpu/gpu/soc/skin, etc.) and takes the maximum value
- Applies smooth temperature zones (with hysteresis) to reduce max caps gradually

### Fan control
- Uses:
  - `/sys/kernel/fan/fan_enable`
  - `/sys/kernel/fan/fan_speed_level`
- Charging rule:
  - if charging: base fan level is **at least 3** (even with screen OFF)
  - otherwise: fan follows temperature, screen OFF => fan off

### Idle / sleep mode
- When screen is OFF and the device is idle, the daemon reduces polling frequency
- Watches background CPU usage: if a process exceeds **15% CPU** while screen is OFF, it exits idle mode

### Notifications
Notifications are posted using `cmd notification post` via `su -lp 2000`.
Used only for:
- charging connected/disconnected (fan mode)
- suspicious background CPU consumers detected during long screen-off (notified after screen turns ON)

## Single-binary icon (no extra files)
The notification icon `src/assets/mora.png` is embedded into the binary (`include_bytes!`) and written to:
`/data/local/tmp/mora.png`
at runtime (no manual copying required).

## Requirements
- Rooted Android device
- LineageOS on `tiro` (or a compatible kernel/userspace)
- Termux + Rust toolchain (if building on-device)

## Build

```sh
cargo build --release
