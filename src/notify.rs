
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::Command,
};

use crate::config::{ICON_DST, ICON_URI};

const ICON_BYTES: &[u8] = include_bytes!("assets/mora.png");

fn sh_escape_single_quotes(s: &str) -> String {
    s.replace('\'', r#"'\''"#)
}

pub fn ensure_icon_on_disk() {
    let dst = Path::new(ICON_DST);

    let need_write = match fs::metadata(dst) {
        Ok(m) => m.len() != ICON_BYTES.len() as u64,
        Err(_) => true,
    };

    if need_write {
        if let Err(e) = fs::write(dst, ICON_BYTES) {
            println!("NOTIFY: icon write fail ({})", e);
            return;
        }
        let _ = fs::set_permissions(dst, fs::Permissions::from_mode(0o644));
        println!("NOTIFY: icon ready ({})", ICON_DST);
    }
}

pub fn post_notification(message: &str) {
    let msg = sh_escape_single_quotes(message);

    let cmd = format!(
        "cmd notification post \
         -i {icon} -I {icon} \
         -S messaging --conversation 'MORA' --message 'M9RA: {msg}' \
         -t 'MORA' 'Tag' 'MORA' >/dev/null 2>&1",
        icon = ICON_URI,
        msg = msg
    );

    let _ = Command::new("su")
        .args(["-lp", "2000", "-c", &cmd])
        .status();
}
