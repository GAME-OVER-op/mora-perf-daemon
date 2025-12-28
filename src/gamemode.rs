
use std::{collections::HashSet, process::Command};

const GAME_LIST_STR: &str = include_str!("data/gamelist.txt");

pub fn load_game_list() -> HashSet<String> {
    GAME_LIST_STR
        .split(|c| c == '|' || c == '\n' || c == '\r' || c == ' ' || c == '\t')
        .filter_map(|s| {
            let t = s.trim();
            if t.is_empty() { None } else { Some(t.to_string()) }
        })
        .collect()
}

fn sh_out(cmd: &str) -> Option<String> {
    let out = Command::new("/system/bin/sh")
        .args(["-c", cmd])
        .output()
        .ok()?;
    if !out.status.success() { return None; }
    String::from_utf8(out.stdout).ok()
}

fn sanitize_pkg(s: &str) -> String {
    let s = s.trim();
    s.trim_matches(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-'))
        .to_string()
}

fn parse_pkg_from_line(line: &str) -> Option<String> {
    for tok in line.split_whitespace() {
        if let Some((pkg, _rest)) = tok.split_once('/') {
            if pkg.contains('.') {
                let p = sanitize_pkg(pkg);
                if p.starts_with("com.") || p.contains('.') {
                    return Some(p);
                }
            }
        }
    }

    if let Some(pos) = line.find("com.") {
        let sub = &line[pos..];
        let end = sub.find('/').or_else(|| sub.find(' ')).unwrap_or(sub.len());
        return Some(sanitize_pkg(&sub[..end]));
    }

    None
}

pub fn get_foreground_package() -> Option<String> {
    if let Some(s) = sh_out("cmd activity get-top-activity 2>/dev/null") {
        for line in s.lines() {
            if let Some(pkg) = parse_pkg_from_line(line) {
                return Some(pkg);
            }
        }
    }

    if let Some(s) = sh_out("dumpsys activity activities 2>/dev/null | grep -m 1 -E 'mResumedActivity|topResumedActivity|ResumedActivity'") {
        for line in s.lines() {
            if let Some(pkg) = parse_pkg_from_line(line) {
                return Some(pkg);
            }
        }
    }

    if let Some(s) = sh_out("dumpsys window windows 2>/dev/null | grep -m 1 -E 'mCurrentFocus|mFocusedApp'") {
        for line in s.lines() {
            if let Some(pkg) = parse_pkg_from_line(line) {
                return Some(pkg);
            }
        }
    }

    None
}

pub fn am_kill_all() {
    let _ = Command::new("/system/bin/am").arg("kill-all").status();
}
