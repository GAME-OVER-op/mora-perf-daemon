
use std::{
    collections::HashMap,
    fs,
    io,
    path::{Path, PathBuf},
};

pub fn read_to_string(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

pub fn read_u64(path: &Path) -> Option<u64> {
    let s = read_to_string(path)?;
    s.trim().parse::<u64>().ok()
}

pub fn read_i32(path: &Path) -> Option<i32> {
    let s = read_to_string(path)?;
    s.trim().parse::<i32>().ok()
}

pub fn write_num(path: &Path, val: u64) -> io::Result<()> {
    fs::write(path, format!("{}\n", val).as_bytes())
}

pub fn write_u64_if_needed(
    path: &Path,
    target: u64,
    cache: &mut HashMap<PathBuf, u64>,
    force_check_current: bool,
) -> io::Result<bool> {
    if !path.exists() {
        return Ok(false);
    }

    if let Some(last) = cache.get(path).copied() {
        if !force_check_current && last == target {
            return Ok(false);
        }
    }

    if force_check_current {
        if let Some(cur) = read_u64(path) {
            if cur == target {
                cache.insert(path.to_path_buf(), target);
                return Ok(false);
            }
        }
    }

    write_num(path, target)?;
    cache.insert(path.to_path_buf(), target);
    Ok(true)
}

pub fn write_str_if_needed(
    path: &Path,
    target: &str,
    cache: &mut HashMap<PathBuf, String>,
    force_check_current: bool,
) -> io::Result<bool> {
    if !path.exists() {
        return Ok(false);
    }

    if let Some(last) = cache.get(path) {
        if !force_check_current && last == target {
            return Ok(false);
        }
    }

    if force_check_current {
        if let Ok(cur) = fs::read_to_string(path) {
            if cur.trim() == target {
                cache.insert(path.to_path_buf(), target.to_string());
                return Ok(false);
            }
        }
    }

    fs::write(path, format!("{}\n", target).as_bytes())?;
    cache.insert(path.to_path_buf(), target.to_string());
    Ok(true)
}
