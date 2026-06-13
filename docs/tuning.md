# Tuning

## Goals
- Stable performance with minimal oscillations
- Fast ramp-up on sudden load spikes
- Slow ramp-down to keep utilization near ~70%
- Thermal protection with smooth caps
- Idle mode when screen is OFF and the device is not used

## Key parameters (current defaults)
- Target utilization: ~70%
- Background CPU alert threshold (screen OFF): 15%
- Idle enter timeout: 10s of inactivity
- Long screen-off window for “suspicious” summary: 30s

## Thermal zones
The daemon reads multiple thermal zones and uses the maximum temperature.
Temperature thresholds and reductions are device-specific.

## Fan thresholds
Fan levels are temperature-based with hysteresis and a charging override:
- charging: min level = 3
- not charging: screen off => fan off
