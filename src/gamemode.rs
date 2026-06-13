use std::{collections::HashSet, fs, path::Path, process::Command};

const TOP_APP_PROCS: &[&str] = &[
    "/dev/cpuset/top-app/cgroup.procs",
    "/dev/cpuset/top-app/tasks",
];

const IGNORE_PACKAGES: &[&str] = &[
    "system_server",
    "com.android.systemui",
    "com.android.launcher",
    "com.android.launcher3",
    "com.google.android.inputmethod.latin",
    "com.android.inputmethod.latin",
    "com.google.android.apps.nexuslauncher",
    "com.termux", // Termux shells often inherit top-app while testing; never a game package.
];

const IGNORE_PROCESS_NAMES: &[&str] = &[
    "sh", "bash", "su", "toybox", "cmd", "dumpsys", "grep", "settings", "ssh-agent",
];

fn sanitize_pkg(s: &str) -> String {
    let s = s.trim();
    s.trim_matches(|c: char| {
        !(c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' || c == ':')
    })
    .to_string()
}

fn package_base(s: &str) -> String {
    sanitize_pkg(s).split(':').next().unwrap_or("").to_string()
}

fn looks_like_package(s: &str) -> bool {
    let p = package_base(s);
    if p.is_empty() || !p.contains('.') {
        return false;
    }
    if IGNORE_PACKAGES.iter().any(|x| p == *x || p.starts_with(&format!("{}.", x))) {
        return false;
    }
    if IGNORE_PROCESS_NAMES.iter().any(|x| p == *x) {
        return false;
    }
    // Android packages generally contain only these characters. Require at least
    // one alphabetic char to avoid odd paths / numeric cgroup fragments.
    p.chars().any(|c| c.is_ascii_alphabetic())
        && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
}

fn read_cmdline(pid: u32) -> Option<String> {
    let data = fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    let first = data.split(|&b| b == 0).next().unwrap_or(&[]);
    if first.is_empty() {
        return None;
    }
    String::from_utf8(first.to_vec()).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

fn tgid_for_tid(tid: u32) -> u32 {
    let status = fs::read_to_string(format!("/proc/{tid}/status")).unwrap_or_default();
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("Tgid:") {
            if let Ok(v) = rest.trim().parse::<u32>() {
                return v;
            }
        }
    }
    tid
}

fn owner_pid_from_cgroup(pid: u32) -> Option<u32> {
    let cg = fs::read_to_string(format!("/proc/{pid}/cgroup")).ok()?;
    for part in cg.split('/') {
        if let Some(rest) = part.strip_prefix("pid_") {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(v) = digits.parse::<u32>() {
                return Some(v);
            }
        }
    }
    None
}

fn candidate_package_for_id(id: u32) -> Option<String> {
    let pid = tgid_for_tid(id);

    if let Some(cmd) = read_cmdline(pid) {
        if looks_like_package(&cmd) {
            return Some(package_base(&cmd));
        }
    }

    if let Some(owner) = owner_pid_from_cgroup(pid) {
        if let Some(cmd) = read_cmdline(owner) {
            if looks_like_package(&cmd) {
                return Some(package_base(&cmd));
            }
        }
    }

    None
}

fn read_top_app_ids(path: &str) -> Vec<u32> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|s| s.parse::<u32>().ok())
        .collect()
}

fn get_top_app_from_cpuset() -> Option<String> {
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();

    for path in TOP_APP_PROCS {
        if !Path::new(path).exists() {
            continue;
        }
        for id in read_top_app_ids(path) {
            if !seen.insert(id) {
                continue;
            }
            if let Some(pkg) = candidate_package_for_id(id) {
                if !candidates.contains(&pkg) {
                    candidates.push(pkg);
                }
            }
        }
        // cgroup.procs is cleaner than tasks. If it gave us a candidate, use it.
        if !candidates.is_empty() && path.ends_with("cgroup.procs") {
            break;
        }
    }

    // Prefer non-keyboard/non-launcher candidates. If multiple remain, the most
    // recently added top-app process is commonly the real foreground app on Android.
    candidates.into_iter().last()
}

fn sh_out(cmd: &str) -> Option<String> {
    let out = Command::new("/system/bin/sh")
        .args(["-c", cmd])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout).ok()
}

fn parse_pkg_from_line(line: &str) -> Option<String> {
    for tok in line.split_whitespace() {
        if let Some((pkg, _rest)) = tok.split_once('/') {
            if looks_like_package(pkg) {
                return Some(package_base(pkg));
            }
        }
    }

    if let Some(pos) = line.find("com.") {
        let sub = &line[pos..];
        let end = sub
            .find('/')
            .or_else(|| sub.find(' '))
            .unwrap_or(sub.len());
        let p = &sub[..end];
        if looks_like_package(p) {
            return Some(package_base(p));
        }
    }

    None
}

fn get_top_app_from_framework_fallback() -> Option<String> {
    // Keep framework calls as a last resort only. On this LineageOS build
    // `cmd activity get-top-activity` is unavailable, so prefer a short dumpsys parse.
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

pub fn get_foreground_package() -> Option<String> {
    get_top_app_from_cpuset().or_else(get_top_app_from_framework_fallback)
}
