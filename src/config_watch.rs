use crate::{
    state::SharedState,
    user_config::{load_or_init, write_config_atomic, UserConfig},
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

/// Poll config.json for changes. On parse/validation errors resets to default.
pub fn spawn(shared: Arc<RwLock<SharedState>>, path: PathBuf) {
    thread::spawn(move || {
        let mut last_mtime: Option<u64> = None;
        loop {
            let mtime = fs::metadata(&path)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs());

            if mtime.is_some() && mtime != last_mtime {
                let cfg = load_or_init(path.as_path());
                {
                    let mut s = shared.write().unwrap();
                    s.config = cfg;
                    s.config_rev = s.config_rev.wrapping_add(1);
                    s.last_config_error = None;
                }
                last_mtime = mtime;
            } else if mtime.is_none() && last_mtime.is_some() {
                // Config was removed; recreate defaults.
                let def = UserConfig::default();
                let _ = write_config_atomic(path.as_path(), &def);
                {
                    let mut s = shared.write().unwrap();
                    s.config = def;
                    s.config_rev = s.config_rev.wrapping_add(1);
                    s.last_config_error = Some("config missing: reset to default".to_string());
                }
                last_mtime = None;
            }

            // Config changes are rare; poll a bit slower to reduce wakeups.
            thread::sleep(Duration::from_millis(1500));
        }
    });
}

/// Apply a new config into shared state and persist to disk.
/// Returns error string for HTTP responses.
pub fn apply_and_persist(
    shared: &Arc<RwLock<SharedState>>,
    path: &Path,
    mut cfg: UserConfig,
) -> Result<(), String> {
    cfg.validate_and_normalize()?;
    write_config_atomic(path, &cfg).map_err(|e| e.to_string())?;
    {
        let mut s = shared.write().unwrap();
        s.config = cfg;
        s.config_rev = s.config_rev.wrapping_add(1);
        s.last_config_error = None;
    }
    Ok(())
}
