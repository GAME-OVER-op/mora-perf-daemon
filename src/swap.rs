use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::unix::fs::{FileTypeExt, PermissionsExt},
    path::Path,
    process::{Command, Stdio},
    thread,
    time::Duration,
};

const ZRAM_MB: u64 = 10_240;
const SWAPFILE_MB: u64 = 2_048;

const SWAPFILE: &str = "/data/swapfile";
const SWAPFILE_TMP: &str = "/data/swapfile.tmp";

const ZRAM_BLOCK: &str = "/dev/block/zram0";
const ZRAM_DEV: &str = "/dev/zram0";

pub fn init_silent() {
    let _ = setup_swap_silent();
}

fn setup_swap_silent() -> io::Result<()> {
    wait_data_writable()?;
    cleanup_zram1();
    setup_zram_10gb();
    setup_swapfile_2gb()?;
    apply_vm_tuning();
    Ok(())
}

fn is_block(path: &str) -> bool {
    fs::metadata(path)
        .map(|m| m.file_type().is_block_device())
        .unwrap_or(false)
}

fn zram_dev() -> Option<&'static str> {
    if is_block(ZRAM_BLOCK) {
        Some(ZRAM_BLOCK)
    } else if is_block(ZRAM_DEV) {
        Some(ZRAM_DEV)
    } else {
        None
    }
}

fn command_ok(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn mkswap(path: &str) -> bool {
    command_ok("mkswap", &[path]) || command_ok("busybox", &["mkswap", path])
}

fn swapoff(path: &str) -> bool {
    command_ok("swapoff", &[path]) || command_ok("busybox", &["swapoff", path])
}

fn swapon(path: &str, priority: i32) -> bool {
    let prio = priority.to_string();
    command_ok("swapon", &["-p", &prio, path])
        || command_ok("busybox", &["swapon", "-p", &prio, path])
}

fn write_str(path: &str, value: &str) -> bool {
    fs::write(path, value.as_bytes()).is_ok()
}

fn active_swap(needle: &str) -> bool {
    fs::read_to_string("/proc/swaps")
        .map(|s| s.lines().any(|line| line.contains(needle)))
        .unwrap_or(false)
}

fn wait_data_writable() -> io::Result<()> {
    let pid = std::process::id();
    let path = format!("/data/.swap_test.{pid}");
    for _ in 0..30 {
        match OpenOptions::new().create(true).write(true).truncate(true).open(&path) {
            Ok(mut f) => {
                let _ = f.write_all(b"1");
                let _ = fs::remove_file(&path);
                return Ok(());
            }
            Err(_) => thread::sleep(Duration::from_secs(2)),
        }
    }
    Err(io::Error::new(io::ErrorKind::Other, "/data is not writable"))
}

fn cleanup_zram1() {
    if Path::new("/sys/block/zram1").exists() {
        let _ = swapoff("/dev/block/zram1");
        let _ = swapoff("/dev/zram1");
        let _ = write_str("/sys/block/zram1/reset", "1\n");
        let _ = write_str("/sys/class/zram-control/hot_remove", "1\n");
    }
}

fn setup_zram_10gb() {
    let Some(dev) = zram_dev() else { return; };
    if !Path::new("/sys/block/zram0").is_dir() {
        return;
    }

    if active_swap("zram0") && !swapoff(dev) {
        return;
    }

    let _ = write_str("/sys/block/zram0/reset", "1\n");

    if let Ok(algos) = fs::read_to_string("/sys/block/zram0/comp_algorithm") {
        for alg in ["lz4", "zstd", "lzo-rle", "lzo"] {
            if algos.split_whitespace().any(|x| x.trim_matches(|c| c == '[' || c == ']') == alg) {
                if write_str("/sys/block/zram0/comp_algorithm", alg) {
                    break;
                }
            }
        }
    }

    let _ = write_str("/sys/block/zram0/reset", "1\n");
    if !write_str("/sys/block/zram0/disksize", &format!("{}M\n", ZRAM_MB)) {
        return;
    }

    if mkswap(dev) {
        let _ = swapon(dev, 100);
    }
}

fn setup_swapfile_2gb() -> io::Result<()> {
    let want_size = SWAPFILE_MB * 1024 * 1024;
    let need_create = fs::metadata(SWAPFILE)
        .map(|m| m.len() != want_size)
        .unwrap_or(true);

    if need_create {
        let _ = fs::remove_file(SWAPFILE_TMP);
        create_zero_file(SWAPFILE_TMP, want_size)?;
        fs::set_permissions(SWAPFILE_TMP, fs::Permissions::from_mode(0o600))?;

        if !mkswap(SWAPFILE_TMP) {
            let _ = fs::remove_file(SWAPFILE_TMP);
            return Err(io::Error::new(io::ErrorKind::Other, "mkswap tmp swapfile failed"));
        }

        let _ = swapoff(SWAPFILE);
        if let Err(e) = fs::rename(SWAPFILE_TMP, SWAPFILE) {
            let _ = fs::remove_file(SWAPFILE_TMP);
            return Err(e);
        }
    } else {
        let _ = swapoff(SWAPFILE);
        fs::set_permissions(SWAPFILE, fs::Permissions::from_mode(0o600))?;
        if !mkswap(SWAPFILE) {
            return Err(io::Error::new(io::ErrorKind::Other, "mkswap swapfile failed"));
        }
    }

    if swapon(SWAPFILE, 10) {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "swapon swapfile failed"))
    }
}

fn create_zero_file(path: &str, size: u64) -> io::Result<()> {
    let mut f = File::create(path)?;
    // Keep the same robust behavior as `dd ... conv=fsync`: actually allocate/write
    // the file, then sync it before it replaces the old swapfile.
    let buf = vec![0u8; 1024 * 1024];
    for _ in 0..SWAPFILE_MB {
        f.write_all(&buf)?;
    }
    f.set_len(size)?;
    f.sync_all()?;
    Ok(())
}

fn apply_vm_tuning() {
    let _ = write_str("/proc/sys/vm/swappiness", "200\n");
    let _ = write_str("/proc/sys/vm/vfs_cache_pressure", "100\n");
    let _ = write_str("/proc/sys/vm/page-cluster", "0\n");
}
