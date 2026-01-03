# mora-perf-daemon

Root performance daemon for **LineageOS on Nubia Red Magic 9 Pro (`tiro`)**:
CPU cpufreq caps, Adreno/KGSL GPU devfreq caps, thermal-based scaling, built-in fan control, idle/sleep mode, **device lighting control**, and Android notifications.

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

### Device lighting (AW22xxx LEDs)
Controls the Red Magic lighting via sysfs, including both **fan ring** and **external/back LEDs**.

- Sysfs base:
  - `/sys/class/leds/aw22xxx_led/*`
- Supports multiple effects (steady / breathe / flow / flashing, etc.) and multiple colors (device presets)
- Can apply lighting per-profile (e.g. Normal / Gaming), and also react to events:
  - charging state
  - notification-triggered external LED scenario

**Important implementation detail:** on this device the fan and external LEDs share the same `effect` interface.
The daemon applies LED updates in a safe order (external → fan) with a small delay to avoid one channel overriding the other.

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

## Local API (API-only)

The daemon exposes a localhost-only HTTP API for a companion Android app:

- `http://127.0.0.1:1004`

This is **API-only mode**:
- Any request **not** under `/api/*` returns an **empty `404`**
- Any `/api/*` request **without a valid token** returns an **empty `404`**
- This makes the port look “dark” in a browser and prevents casual probing

### Authentication
Every API request must include a token using **one** of these headers:
- `Authorization: Bearer <token>`
- `X-Api-Key: <token>`

The token is stored in `config.json` as `api_token`.
If `api_token` is missing/empty, the daemon generates a random token from `/dev/urandom` and persists it.

### Endpoints
- `GET /api/state` — runtime state (active profile, temps, modes, fan/led state, etc.)
- `GET /api/config` — current effective config
- `POST /api/save` — apply UI/app settings and persist to `config.json`

Quick test:
```sh
TOKEN="YOUR_TOKEN_HERE"
curl -s -H "X-Api-Key: $TOKEN" http://127.0.0.1:1004/api/state
```

## Configuration

Default config path used by the daemon:

* `/data/adb/modules/mora_perf_deamon/config/config.json`

The daemon hot-reloads the config when the file changes.

## Magisk module

This project is intended to be deployed as a **Magisk module**:

* daemon runs as root on boot
* config is stored under the module directory (`/data/adb/modules/...`)

(Exact module files/structure may vary depending on your packaging.)

## Requirements

* Rooted Android device (Magisk)
* LineageOS on `tiro` (or a compatible kernel/userspace)
* Termux + Rust toolchain (if building on-device)

## Build

```sh
cargo build --release
```
