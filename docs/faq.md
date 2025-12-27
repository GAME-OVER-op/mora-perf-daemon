# FAQ

## Q: GPU utilization is always 0%
A: Some kernels do not expose `gpu_busy_percentage`. The daemon falls back to `gpubusy`.
Check:
- `/sys/class/kgsl/kgsl-3d0/gpu_busy_percentage`
- `/sys/class/kgsl/kgsl-3d0/gpubusy`

## Q: Fan does not spin
A: Verify sysfs nodes:
- `/sys/kernel/fan/fan_enable`
- `/sys/kernel/fan/fan_speed_level`
Try manually:
`echo 1 > /sys/kernel/fan/fan_enable; echo 3 > /sys/kernel/fan/fan_speed_level`

## Q: Will this work on other phones?
A: It is tuned for `tiro`. Porting requires updating frequency tables, sysfs paths, and thresholds.
