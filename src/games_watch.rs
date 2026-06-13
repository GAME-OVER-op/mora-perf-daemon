use crate::{
    games::{apply_updatable_driver_apps, load_or_init, write_games_atomic, GamesFile, GamesRuntime},
    state::SharedState,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

fn mtime_secs(path: &Path) -> Option<u64> {
    fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

/// Poll games.json for changes. On parse errors resets to empty list.
pub fn spawn(shared: Arc<RwLock<SharedState>>, path: PathBuf) {
    thread::spawn(move || {
        let mut last_mtime = mtime_secs(path.as_path());

        loop {
            let mtime = mtime_secs(path.as_path());

            if mtime != last_mtime {
                let (rt, err) = load_or_init(path.as_path());
                let driver = rt.driver_string.clone();
                {
                    let mut s = shared.write().unwrap();
                    s.games = rt;
                    s.games_rev = s.games_rev.wrapping_add(1);
                    s.last_games_error = err;
                }
                apply_updatable_driver_apps(&driver);
                last_mtime = mtime_secs(path.as_path());
            }

            // Changes are rare; poll slowly.
            thread::sleep(Duration::from_millis(8000));
        }
    });
}

/// Apply a new games file into shared state and persist to disk.
/// Also syncs Android updatable game driver list.
pub fn apply_and_persist(
    shared: &Arc<RwLock<SharedState>>,
    path: &Path,
    file: GamesFile,
) -> Result<(), String> {
    let rt = GamesRuntime::from_file(file);
    write_games_atomic(path, &rt.file).map_err(|e| e.to_string())?;

    {
        let mut s = shared.write().unwrap();
        s.games = rt.clone();
        s.games_rev = s.games_rev.wrapping_add(1);
        s.last_games_error = None;
    }

    apply_updatable_driver_apps(&rt.driver_string);
    Ok(())
}
