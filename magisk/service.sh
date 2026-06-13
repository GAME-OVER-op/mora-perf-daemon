#!/system/bin/sh

# --- дождаться полного старта системы
while [[ "$(getprop sys.boot_completed)" != "1" ]]; do
  sleep 5
done

setprop debug.graphics.game_default_frame_rate.disabled true
iw dev wlan0 set power_save off

setsid perf_daemon  >/dev/null >/dev/null 2>&1 &
