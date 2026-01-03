use crate::{
    config_watch,
    mem::read_vmrss_kb,
    state::SharedState,
    user_config::{
        FanLedSetting, NotificationsConfig, ProfileConfig, ProfileType, UserConfig,
    },
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    thread,
};
use tiny_http::{Header, Method, Response, Server, StatusCode};

const BIND_ADDR: &str = "127.0.0.1:1004";

// User will replace this later. We ship a placeholder so the project compiles.
const RED_PNG: &[u8] = include_bytes!("assets/red.png");

// UI layout/styling uses device-like palette (black/red/yellow/blue)
// based on the user's provided `script/index.html`.
const STYLE: &str = r#"
:root{
  --bg:#050506;
  --surface: rgba(12,12,16,.78);
  --surface2: rgba(12,12,16,.58);
  --border: rgba(255,255,255,.08);
  --borderHot: rgba(255,100,120,.18);

  --text-primary: rgba(245,245,250,.92);
  --text-secondary: rgba(245,245,250,.74);
  --muted: rgba(245,245,250,.62);

  --accent-red: #ff3b5c;
  --accent-red-rgb: 255,59,92;
  --accent-blue: #3a86ff;
  --accent-blue-rgb: 58,134,255;
  --accent-yellow: #ffd86b;
  --accent-yellow-rgb: 255,216,107;

  --primary-color: var(--accent-red);
  --primary-color-rgb: var(--accent-red-rgb);

  --radius: 22px;
  --radius-sm: 14px;
  --shadow: 0 18px 60px rgba(0,0,0,.55);
  --sidebar-compact-width: 64px;
  --sidebar-full-width: 280px;
  --transition: 260ms cubic-bezier(.2,.9,.2,1);
}

[data-theme="light"]{
  --bg:#f6f7f9;
  --surface: rgba(255,255,255,.92);
  --surface2: rgba(255,255,255,.70);
  --border: rgba(0,0,0,.10);
  --borderHot: rgba(255,59,92,.18);
  --text-primary: rgba(10,12,14,.92);
  --text-secondary: rgba(10,12,14,.72);
  --muted: rgba(10,12,14,.55);
  --shadow: 0 18px 60px rgba(0,0,0,.14);
}

*{ box-sizing:border-box; margin:0; padding:0; }
html,body{ height:100%; }

body{
  font-family: Inter, Segoe UI, Roboto, system-ui;
  background: var(--bg);
  color: var(--text-primary);
  overflow-x:hidden;
  line-height:1.5;
}

/* Background paints */
.bg{
  position: fixed; inset:0; z-index:0; pointer-events:none;
  background:
    radial-gradient(1100px 800px at 18% 8%, rgba(var(--accent-red-rgb),.10), transparent 62%),
    radial-gradient(900px 700px at 82% 0%, rgba(255,107,125,.08), transparent 58%),
    radial-gradient(1100px 900px at 62% 92%, rgba(var(--accent-yellow-rgb),.07), transparent 62%),
    radial-gradient(900px 700px at 80% 84%, rgba(var(--accent-blue-rgb),.06), transparent 60%),
    var(--bg);
}

.app-container{ position:relative; z-index:1; display:flex; min-height:100vh; width:100%; }

.sidebar {
  position: fixed; left:0; top:0; height:100vh; z-index:1000;
  display:flex; flex-direction:column;
  transition: transform var(--transition), width var(--transition);
  width: var(--sidebar-compact-width);
  background: var(--surface);
  border-right: 1px solid var(--border);
  box-shadow: var(--shadow);
  backdrop-filter: blur(10px);
  overflow-x:hidden; overflow-y:auto;
}

.sidebar.open { width: var(--sidebar-full-width); }

.sidebar-header{
  display:flex; align-items:center; gap:12px;
  padding: 20px 16px;
  border-bottom: 1px solid var(--border);
  min-height:80px;
}

.logo{
  width:44px; height:44px; border-radius:12px;
  display:flex; align-items:center; justify-content:center;
  flex-shrink:0; overflow:hidden;
  background: transparent; /* no colorful frame */
}

.logo img{ width: 34px; height: 34px; object-fit: contain; filter: none; }

.title-wrap{ overflow:hidden; flex:1; }
.title{ font-weight:600; font-size:1.1rem; white-space:nowrap; opacity:0; transform: translateX(-8px);
  transition: opacity var(--transition), transform var(--transition); color: var(--text-primary);
}
.subtitle{ font-size:0.85rem; color: var(--muted); white-space:nowrap; opacity:0; transform: translateX(-8px);
  transition: opacity calc(var(--transition) + 80ms), transform calc(var(--transition) + 80ms);
}
.sidebar.open .title, .sidebar.open .subtitle{ opacity:1; transform:none; }

.hamburger{
  position: fixed; right:20px; bottom:20px; z-index:2000;
  width:56px; height:56px; border-radius:50%;
  background: var(--primary-color); color:#fff; border:none;
  display:flex; align-items:center; justify-content:center;
  cursor:pointer; box-shadow: 0 4px 20px rgba(58,134,255,0.30);
  transition: all var(--transition);
}
.hamburger{ box-shadow: 0 10px 30px rgba(var(--primary-color-rgb),0.28); }
.hamburger.hidden{ opacity:0; transform: scale(0); pointer-events:none; }

.nav{ padding:16px; display:flex; flex-direction:column; gap:8px; flex:1; }
.nav-item{
  display:flex; align-items:center; gap:12px; padding: 14px;
  border-radius: 10px; color: var(--text-primary); text-decoration:none; cursor:pointer;
  transition: background var(--transition), transform var(--transition);
}
.nav-item:hover{ background: rgba(var(--primary-color-rgb), 0.10); transform: translateX(2px); }
.nav-item.active{ background: rgba(var(--primary-color-rgb), 0.16); border: 1px solid rgba(var(--primary-color-rgb),0.24); }
.nav-item .ico{ width:28px; text-align:center; font-size: 1.1rem; flex-shrink:0; }
.nav-item span{
  white-space: nowrap; opacity:0; transform: translateX(-6px);
  transition: opacity var(--transition), transform var(--transition);
  font-weight: 500;
}
.sidebar.open .nav-item span{ opacity:1; transform:none; }

.sidebar-footer{ padding:16px; border-top:1px solid var(--border); }
.toggle-row{ display:flex; align-items:center; justify-content:space-between; gap:12px; }
.toggle-label{ font-size:0.95rem; color: var(--text-secondary); opacity:0; transform: translateX(-6px);
  transition: opacity var(--transition), transform var(--transition);
}
.sidebar.open .toggle-label{ opacity:1; transform:none; }
.theme-toggle{
  width:44px; height:24px; border-radius:999px;
  background: rgba(255,255,255,0.08);
  border: 1px solid var(--border);
  position: relative; cursor:pointer; flex-shrink:0;
}
[data-theme="light"] .theme-toggle{ background: rgba(0,0,0,0.08); }
.theme-toggle::after{
  content:""; width:18px; height:18px; border-radius:50%;
  position:absolute; left:2px; top:2px;
  background: var(--surface);
  box-shadow: 0 4px 10px rgba(0,0,0,0.15);
  transition: transform var(--transition);
}
[data-theme="dark"] .theme-toggle::after{ transform: translateX(20px); }

.overlay{
  position: fixed; inset:0; background: rgba(0,0,0,0.25);
  z-index: 900; opacity:0; pointer-events:none;
  transition: opacity var(--transition);
}
.overlay.show{ opacity:1; pointer-events:auto; }

.main{
  margin-left: var(--sidebar-compact-width);
  width: 100%;
  padding: 24px;
  transition: margin-left var(--transition);
}
.sidebar.open ~ .main{ margin-left: var(--sidebar-full-width); }

@media (max-width: 760px){
  .main{ margin-left: 0; padding: 18px; }
  .sidebar{ transform: translateX(-100%); width: var(--sidebar-full-width); }
  .sidebar.open{ transform: translateX(0); }
  .sidebar.open ~ .main{ margin-left: 0; }
}

.page-title{ display:flex; align-items:flex-end; justify-content:space-between; gap:12px; margin-bottom: 16px; }
.page-title h1{ font-size: 1.4rem; letter-spacing: 0.2px; }
.page-title p{ color: var(--muted); font-size: 0.95rem; }

.grid{ display:grid; grid-template-columns: repeat(12, 1fr); gap: 14px; }
@media (max-width: 980px){ .grid{ grid-template-columns: repeat(1, 1fr); } }

.card{
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
  padding: 16px;
}

.card h2{ font-size: 0.95rem; margin-bottom: 10px; color: var(--text-secondary); }

.kv{ display:flex; justify-content:space-between; gap:10px; padding: 8px 0; border-bottom: 1px solid var(--border); }
.kv:last-child{ border-bottom:none; }
.k{ color: var(--muted); }
.v{ font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace; }

.row{ display:flex; gap:12px; flex-wrap:wrap; align-items:flex-end; }
label{ display:block; font-size: 0.80rem; letter-spacing: .08em; text-transform: uppercase; color: var(--muted); margin-bottom: 6px; }
select,input{
  background: var(--surface2);
  color: var(--text-primary);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 10px 12px;
  min-width: 160px;
}
input[type="number"]{ min-width: 120px; }
button{
  background: rgba(var(--primary-color-rgb), 0.14);
  border: 1px solid rgba(var(--primary-color-rgb), 0.35);
  color: var(--text-primary);
  border-radius: 999px;
  padding: 10px 14px;
  cursor:pointer;
  font-weight: 600;
}
button:hover{ background: rgba(var(--primary-color-rgb), 0.20); }
.hint{ color: var(--muted); font-size: 0.90rem; }
.badge{ display:inline-block; padding: 6px 10px; border-radius: 999px; border: 1px solid var(--borderHot);
  background: rgba(var(--primary-color-rgb),0.08);
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  font-size: 0.85rem;
}
"#;

fn ok_html(s: String) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(s)
        .with_header(Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap())
}

fn ok_json(v: Value) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(v.to_string())
        .with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap())
}

fn bad(code: u16, msg: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(msg).with_status_code(StatusCode(code))
}

fn empty_404() -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_data(Vec::<u8>::new()).with_status_code(StatusCode(404))
}

// tiny_http's HeaderField::equiv expects a `&'static str`.
// We only query a fixed set of well-known headers, so keep this API static.
fn header_value(req: &tiny_http::Request, name: &'static str) -> Option<String> {
    req.headers()
        .iter()
        .find(|h| h.field.equiv(name))
        // tiny_http stores header values as AsciiString; normalize to String.
        .map(|h| h.value.to_string())
}

/// Authorization model:
/// - Only /api/* endpoints exist.
/// - Requests must include a valid token.
/// - If token is missing/invalid, we respond with an empty 404 (so opening the port in a browser shows nothing).
///
/// Supported headers:
/// - Authorization: Bearer <token>
/// - X-Api-Key: <token>
fn is_authorized(req: &tiny_http::Request, shared: &Arc<RwLock<SharedState>>) -> bool {
    let token = { shared.read().unwrap().config.api_token.clone() };
    if token.trim().is_empty() {
        // Token must exist (it should be auto-generated & persisted on config load).
        // If it doesn't, deny access and return an empty 404 to keep the port "dark".
        return false;
    }

    if let Some(v) = header_value(req, "Authorization") {
        let v = v.trim();
        if v == token {
            return true;
        }
        if let Some(rest) = v.strip_prefix("Bearer ") {
            if rest.trim() == token {
                return true;
            }
        }
        if let Some(rest) = v.strip_prefix("Token ") {
            if rest.trim() == token {
                return true;
            }
        }
    }

    if let Some(v) = header_value(req, "X-Api-Key") {
        if v.trim() == token {
            return true;
        }
    }

    false
}

fn read_body(req: &mut tiny_http::Request) -> Vec<u8> {
    let mut buf = Vec::new();
    let _ = req.as_reader().read_to_end(&mut buf);
    buf
}

fn build_state_json(shared: &Arc<RwLock<SharedState>>) -> Value {
    let s = shared.read().unwrap();

    let to_c = |mc: Option<i32>| mc.map(|v| (v as f64) / 1000.0);

    let ext_active = s.leds.external_active;
    let now = std::time::Instant::now();
    let ext_ends_in = s.leds.external_ends_at.map(|t| {
        if t > now {
            (t - now).as_secs()
        } else {
            0
        }
    });

    json!({
        "temps": {
            "cpu": to_c(s.info.cpu_avg_mc),
            "gpu": to_c(s.info.gpu_avg_mc),
            "soc": to_c(s.info.soc_mc),
            "batt": to_c(s.info.batt_mc)
        },
        "zone": {
            "name": s.info.temp_zone.clone(),
            "reduce_percent": s.info.reduce_percent
        },
        "screen_on": s.info.screen_on,
        "charging": {
            "hw": s.info.charging,
            "enabled": s.info.charging_enabled,
            "effective": s.info.charging_effective
        },
        "game_mode": s.info.game_mode,
        "idle_mode": s.info.idle_mode,
        "active_profile": s.info.active_profile.clone(),
        "led_profile": s.info.led_profile.clone(),
        "leds": {
            "base_external_desired": s.leds.base_external_desired.clone(),
            "base_external_last_applied": s.leds.base_external_last_applied.clone(),
            "fan_desired": s.leds.fan_desired.clone(),
            "fan_last_applied": s.leds.fan_last_applied.clone(),
            "external": {
                "active": ext_active,
                "setting": s.leds.external_setting.clone(),
                "stop_kind": s.leds.external_stop_kind,
                "ends_in_sec": ext_ends_in
            }
        },
        "mem": {
            "VmRSS_kb": read_vmrss_kb()
        },
        "config_rev": s.config_rev,
        "last_config_error": s.last_config_error.clone()
    })
}

fn page_app() -> String {
    let html = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width,initial-scale=1,maximum-scale=5" />
  <title>mora</title>
  <style>#STYLE#</style>
</head>
<body>
<div class="bg"></div>
<div class="app-container">
  <div class="sidebar" id="sidebar">
    <div class="sidebar-header">
      <div class="logo"><img src="/assets/red.png" alt="" /></div>
      <div class="title-wrap">
        <div class="title">mora</div>
        <div class="subtitle">status panel</div>
      </div>
    </div>
    <div class="nav">
      <a class="nav-item active" id="nav_status"><div class="ico">●</div><span>Status</span></a>
      <a class="nav-item" id="nav_profiles"><div class="ico">●</div><span>Profiles</span></a>
    </div>
    <div class="sidebar-footer">
      <div class="toggle-row">
        <div class="toggle-label">Theme</div>
        <div class="theme-toggle" id="theme_toggle" title="theme"></div>
      </div>
      <div style="margin-top:12px" class="hint" id="cfg_err"></div>
    </div>
  </div>

  <div class="overlay" id="overlay"></div>

  <main class="main">
    <section id="section_status">
      <div class="page-title">
        <div>
          <h1>Status</h1>
          <p>updates in real time</p>
        </div>
        <div class="badge" id="badge_profile">?</div>
      </div>

      <div class="grid">
        <div class="card" style="grid-column: span 4;">
          <h2>Temperatures</h2>
          <div class="kv"><div class="k">CPU avg</div><div class="v" id="t_cpu">?</div></div>
          <div class="kv"><div class="k">GPU avg</div><div class="v" id="t_gpu">?</div></div>
          <div class="kv"><div class="k">SOC</div><div class="v" id="t_soc">?</div></div>
          <div class="kv"><div class="k">Battery</div><div class="v" id="t_batt">?</div></div>
        </div>

        <div class="card" style="grid-column: span 4;">
          <h2>States</h2>
          <div class="kv"><div class="k">Zone</div><div class="v" id="st_zone">?</div></div>
          <div class="kv"><div class="k">Reduce</div><div class="v" id="st_reduce">?</div></div>
          <div class="kv"><div class="k">Screen</div><div class="v" id="st_screen">?</div></div>
          <div class="kv"><div class="k">Charging</div><div class="v" id="st_chg">?</div></div>
          <div class="kv"><div class="k">Game</div><div class="v" id="st_game">?</div></div>
          <div class="kv"><div class="k">Idle</div><div class="v" id="st_idle">?</div></div>
          <div class="kv"><div class="k">LED profile</div><div class="v" id="st_ledprof">?</div></div>
        </div>

        <div class="card" style="grid-column: span 4;">
          <h2>Lighting / RAM</h2>
          <div class="kv"><div class="k">Fan LED</div><div class="v" id="st_fan">?</div></div>
          <div class="kv"><div class="k">External LED</div><div class="v" id="st_ext">?</div></div>
          <div class="kv"><div class="k">VmRSS</div><div class="v" id="st_mem">?</div></div>
        </div>
      </div>
    </section>

    <section id="section_profiles" style="display:none;">
      <div class="page-title">
        <div>
          <h1>Profiles</h1>
          <p>enable/disable and parameters</p>
        </div>
        <div class="hint" id="save_msg"></div>
      </div>

      <div class="grid">
        <div class="card" style="grid-column: span 6;">
          <h2>Charging</h2>
          <div class="row">
            <div>
              <label>Enabled</label>
              <select id="c_enabled">
                <option value="true">ON</option>
                <option value="false">OFF</option>
              </select>
            </div>
            <div>
              <label>Fan LED</label>
              <select id="c_fan_enabled">
                <option value="true">ON</option>
                <option value="false">OFF</option>
              </select>
            </div>
          </div>
          <div class="row" style="margin-top:12px;">
            <div>
              <label>Mode</label>
              <select id="c_fan_mode">
                <option value="off">off</option>
                <option value="flow">flow</option>
                <option value="breathe">breathe</option>
                <option value="flashing">flashing</option>
                <option value="steady">steady</option>
              </select>
            </div>
            <div>
              <label>Color</label>
              <select id="c_fan_color">
                <option value="rose">rose</option>
                <option value="yellow">yellow</option>
                <option value="green">green</option>
                <option value="blue">blue</option>
                <option value="cyan">cyan</option>
                <option value="purple">purple</option>
                <option value="orange">orange</option>
                <option value="mixed_1">mixed_1</option>
                <option value="mixed_2">mixed_2</option>
                <option value="mixed_3">mixed_3</option>
                <option value="mixed_4">mixed_4</option>
                <option value="mixed_5">mixed_5</option>
                <option value="mixed_6">mixed_6</option>
                <option value="mixed_7">mixed_7</option>
              </select>
            </div>
          </div>
          <div class="hint" style="margin-top:10px;">Turning this OFF makes it behave as if charging is not connected.</div>
        </div>

        <div class="card" style="grid-column: span 6;">
          <h2>Notifications → external LED</h2>
          <div class="row">
            <div>
              <label>Enabled</label>
              <select id="n_enabled">
                <option value="true">ON</option>
                <option value="false">OFF</option>
              </select>
            </div>
            <div>
              <label>Stop</label>
              <select id="n_stop">
                <option value="until_screen_on">UntilScreenOn</option>
                <option value="for_seconds">ForSeconds(N)</option>
              </select>
            </div>
            <div>
              <label>Seconds</label>
              <input id="n_secs" type="number" min="1" max="3600" value="10" />
            </div>
          </div>
          <div class="row" style="margin-top:12px;">
            <div>
              <label>Mode</label>
              <select id="n_mode">
                <option value="steady">steady</option>
                <option value="breathe">breathe</option>
                <option value="flashing">flashing</option>
              </select>
            </div>
            <div>
              <label>Color</label>
              <select id="n_color">
                <option value="red">red</option>
                <option value="orange">orange</option>
                <option value="yellow">yellow</option>
                <option value="green">green</option>
                <option value="cyan">cyan</option>
                <option value="blue">blue</option>
                <option value="purple">purple</option>
                <option value="pink">pink</option>
                <option value="multi" style="display:none">multi</option>
                <option value="white" style="display:none">white</option>
              </select>
            </div>
          </div>
        </div>

        <div class="card" style="grid-column: span 6;">
          <h2>Normal</h2>
          <div class="row">
            <div>
              <label>Enabled</label>
              <select id="p_n_enabled">
                <option value="true">ON</option>
                <option value="false">OFF</option>
              </select>
            </div>
            <div>
              <label>Fan LED</label>
              <select id="p_n_fan_enabled">
                <option value="true">ON</option>
                <option value="false">OFF</option>
              </select>
            </div>
            <div>
              <label>External LED</label>
              <select id="p_n_ext_enabled">
                <option value="true">ON</option>
                <option value="false">OFF</option>
              </select>
            </div>
          </div>
          <div class="row" style="margin-top:12px;">
            <div>
              <label>Fan Mode</label>
              <select id="p_n_mode">
                <option value="off">off</option>
                <option value="flow">flow</option>
                <option value="breathe">breathe</option>
                <option value="flashing">flashing</option>
                <option value="steady">steady</option>
              </select>
            </div>
            <div>
              <label>Fan Color</label>
              <select id="p_n_color">
                <option value="rose">rose</option>
                <option value="yellow">yellow</option>
                <option value="green">green</option>
                <option value="blue">blue</option>
                <option value="cyan">cyan</option>
                <option value="purple">purple</option>
                <option value="orange">orange</option>
                <option value="mixed_1">mixed_1</option>
                <option value="mixed_2">mixed_2</option>
                <option value="mixed_3">mixed_3</option>
                <option value="mixed_4">mixed_4</option>
                <option value="mixed_5">mixed_5</option>
                <option value="mixed_6">mixed_6</option>
                <option value="mixed_7">mixed_7</option>
              </select>
            </div>
          </div>
          <div class="row" style="margin-top:12px;">
            <div>
              <label>Ext Mode</label>
              <select id="p_n_ext_mode">
                <option value="steady">steady</option>
                <option value="breathe">breathe</option>
                <option value="flashing">flashing</option>
              </select>
            </div>
            <div>
              <label>Ext Color</label>
              <select id="p_n_ext_color">
                <option value="red">red</option>
                <option value="orange">orange</option>
                <option value="yellow">yellow</option>
                <option value="green">green</option>
                <option value="cyan">cyan</option>
                <option value="blue">blue</option>
                <option value="purple">purple</option>
                <option value="pink">pink</option>
                <option value="multi" style="display:none">multi</option>
                <option value="white" style="display:none">white</option>
              </select>
            </div>
          </div>
        </div>

        <div class="card" style="grid-column: span 6;">
          <h2>Gaming</h2>
          <div class="row">
            <div>
              <label>Enabled</label>
              <select id="p_g_enabled">
                <option value="true">ON</option>
                <option value="false">OFF</option>
              </select>
            </div>
            <div>
              <label>Fan LED</label>
              <select id="p_g_fan_enabled">
                <option value="true">ON</option>
                <option value="false">OFF</option>
              </select>
            </div>
            <div>
              <label>External LED</label>
              <select id="p_g_ext_enabled">
                <option value="true">ON</option>
                <option value="false">OFF</option>
              </select>
            </div>
          </div>
          <div class="row" style="margin-top:12px;">
            <div>
              <label>Fan Mode</label>
              <select id="p_g_mode">
                <option value="off">off</option>
                <option value="flow">flow</option>
                <option value="breathe">breathe</option>
                <option value="flashing">flashing</option>
                <option value="steady">steady</option>
              </select>
            </div>
            <div>
              <label>Fan Color</label>
              <select id="p_g_color">
                <option value="rose">rose</option>
                <option value="yellow">yellow</option>
                <option value="green">green</option>
                <option value="blue">blue</option>
                <option value="cyan">cyan</option>
                <option value="purple">purple</option>
                <option value="orange">orange</option>
                <option value="mixed_1">mixed_1</option>
                <option value="mixed_2">mixed_2</option>
                <option value="mixed_3">mixed_3</option>
                <option value="mixed_4">mixed_4</option>
                <option value="mixed_5">mixed_5</option>
                <option value="mixed_6">mixed_6</option>
                <option value="mixed_7">mixed_7</option>
              </select>
            </div>
          </div>
          <div class="row" style="margin-top:12px;">
            <div>
              <label>Ext Mode</label>
              <select id="p_g_ext_mode">
                <option value="steady">steady</option>
                <option value="breathe">breathe</option>
                <option value="flashing">flashing</option>
              </select>
            </div>
            <div>
              <label>Ext Color</label>
              <select id="p_g_ext_color">
                <option value="red">red</option>
                <option value="orange">orange</option>
                <option value="yellow">yellow</option>
                <option value="green">green</option>
                <option value="cyan">cyan</option>
                <option value="blue">blue</option>
                <option value="purple">purple</option>
                <option value="pink">pink</option>
                <option value="multi" style="display:none">multi</option>
                <option value="white" style="display:none">white</option>
              </select>
            </div>
          </div>
        </div>
      </div>
    </section>
  </main>
</div>

<button class="hamburger" id="hamburger" aria-label="menu">☰</button>

<script>
const $ = (id)=>document.getElementById(id);
const sidebar = $('sidebar');
const overlay = $('overlay');
const hamburger = $('hamburger');
const extModeSel = $('p_n_ext_mode');
const extColorSel = $('p_n_ext_color');

function updateExtColorsByMode(){
  if(!extModeSel || !extColorSel) return;
  const mode = extModeSel.value;
  const opts = Array.from(extColorSel.options);

  // On this firmware:
  // - steady/breathe: orange slot is not present (idx0/idx1 are red/violet)
  // - flashing: red slot is not present (idx0/idx1 are orange/pink)
  opts.forEach(o=>{
    if(o.value==='orange'){
      o.hidden = (mode!=='flashing');
      o.style.display = o.hidden ? 'none' : '';
    }
    if(o.value==='red'){
      o.hidden = (mode==='flashing');
      o.style.display = o.hidden ? 'none' : '';
    }
  });

  // If current selection became hidden, pick the first visible option
  const cur = extColorSel.selectedOptions[0];
  if(cur && (cur.hidden || cur.style.display==='none')){
    const first = opts.find(o=>!(o.hidden || o.style.display==='none'));
    if(first) extColorSel.value = first.value;
  }
}

if(extModeSel){
  extModeSel.addEventListener('change', updateExtColorsByMode);
}

function openSidebar(){ sidebar.classList.add('open'); overlay.classList.add('show'); }
function closeSidebar(){ sidebar.classList.remove('open'); overlay.classList.remove('show'); }

hamburger.addEventListener('click', ()=>{
  if(sidebar.classList.contains('open')) closeSidebar(); else openSidebar();
});
overlay.addEventListener('click', closeSidebar);

function setPage(p){
  $('section_status').style.display = (p==='status') ? '' : 'none';
  $('section_profiles').style.display = (p==='profiles') ? '' : 'none';
  $('nav_status').classList.toggle('active', p==='status');
  $('nav_profiles').classList.toggle('active', p==='profiles');
  if(window.innerWidth <= 760) closeSidebar();
}

$('nav_status').addEventListener('click', ()=>setPage('status'));
$('nav_profiles').addEventListener('click', ()=>setPage('profiles'));

// theme
function applyTheme(t){
  document.documentElement.setAttribute('data-theme', t);
  localStorage.setItem('theme', t);
}
const savedTheme = localStorage.getItem('theme') || 'dark';
applyTheme(savedTheme);
$('theme_toggle').addEventListener('click', ()=>{
  const cur = document.documentElement.getAttribute('data-theme') || 'dark';
  applyTheme(cur==='dark' ? 'light' : 'dark');
});

// helpers
function fTemp(x){
  if(x===null || x===undefined) return '?';
  return x.toFixed(1)+'C';
}

let lastCfgRev = -1;
let saveTimer = null;
let saving = false;

function valBool(id){ return $(id).value === 'true'; }
function setBool(id, b){ $(id).value = b ? 'true' : 'false'; }

function applyCfg(cfg){
  if(!cfg) return;
  if(cfg.charging){
    setBool('c_enabled', !!cfg.charging.enabled);
    const fanOn = !!cfg.charging.fan_led;
    setBool('c_fan_enabled', fanOn);
    if(cfg.charging.fan_led){
      $('c_fan_mode').value = cfg.charging.fan_led.mode || 'off';
      $('c_fan_color').value = cfg.charging.fan_led.color || 'mixed_7';
    }
  }

  if(cfg.notifications){
    setBool('n_enabled', !!cfg.notifications.enabled);
    $('n_stop').value = (cfg.notifications.stop_condition && cfg.notifications.stop_condition.type) ? cfg.notifications.stop_condition.type : 'until_screen_on';
    $('n_secs').value = Number(cfg.notifications.for_seconds || 10);
    if(cfg.notifications.external_led){
      $('n_mode').value = cfg.notifications.external_led.mode || 'flashing';
      $('n_color').value = cfg.notifications.external_led.color || 'blue';
    }
  }

  const n = (cfg.profiles && cfg.profiles.normal) ? cfg.profiles.normal : null;
  const g = (cfg.profiles && cfg.profiles.gaming) ? cfg.profiles.gaming : null;
  if(n){
    setBool('p_n_enabled', !!n.enabled);
    setBool('p_n_fan_enabled', !!n.fan_led);
    if(n.fan_led){ $('p_n_mode').value = n.fan_led.mode || 'off'; $('p_n_color').value = n.fan_led.color || 'mixed_7'; }
    setBool('p_n_ext_enabled', !!n.external_led);
    if(n.external_led){ $('p_n_ext_mode').value = n.external_led.mode || 'flashing'; $('p_n_ext_color').value = n.external_led.color || 'blue'; }
  }
  if(g){
    setBool('p_g_enabled', !!g.enabled);
    setBool('p_g_fan_enabled', !!g.fan_led);
    if(g.fan_led){ $('p_g_mode').value = g.fan_led.mode || 'off'; $('p_g_color').value = g.fan_led.color || 'mixed_7'; }
    setBool('p_g_ext_enabled', !!g.external_led);
    if(g.external_led){ $('p_g_ext_mode').value = g.external_led.mode || 'flashing'; $('p_g_ext_color').value = g.external_led.color || 'blue'; }
  }
}

function collectPayload(){
  return {
    charging: {
      enabled: valBool('c_enabled'),
      fan_enabled: valBool('c_fan_enabled'),
      fan_led: { mode: $('c_fan_mode').value, color: $('c_fan_color').value }
    },
    notifications: {
      enabled: valBool('n_enabled'),
      stop_condition: { type: $('n_stop').value },
      for_seconds: Number($('n_secs').value || 10),
      external_led: { mode: $('n_mode').value, color: $('n_color').value }
    },
    profiles: {
      normal: {
        enabled: valBool('p_n_enabled'),
        fan_enabled: valBool('p_n_fan_enabled'),
        fan_led: { mode: $('p_n_mode').value, color: $('p_n_color').value },
        ext_enabled: valBool('p_n_ext_enabled'),
        external_led: { mode: $('p_n_ext_mode').value, color: $('p_n_ext_color').value }
      },
      gaming: {
        enabled: valBool('p_g_enabled'),
        fan_enabled: valBool('p_g_fan_enabled'),
        fan_led: { mode: $('p_g_mode').value, color: $('p_g_color').value },
        ext_enabled: valBool('p_g_ext_enabled'),
        external_led: { mode: $('p_g_ext_mode').value, color: $('p_g_ext_color').value }
      }
    }
  };
}

function scheduleSave(){
  if(saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(doSave, 450);
}

async function doSave(){
  if(saving) return;
  saving = true;
  $('save_msg').textContent = 'Saving...';
  try{
    const payload = collectPayload();
    const r = await fetch('/api/save', { method:'POST', headers:{'Content-Type':'application/json'}, body: JSON.stringify(payload)});
    const t = await r.text();
    if(!r.ok){
      $('save_msg').textContent = 'Error: ' + t;
    } else {
      $('save_msg').textContent = 'ok';
      setTimeout(()=>{ if($('save_msg').textContent==='ok') $('save_msg').textContent=''; }, 1000);
    }
  }catch(e){
    $('save_msg').textContent = 'Error: ' + String(e);
  }
  saving = false;
}

async function refreshCfg(){
  try{
    const r = await fetch('/api/config');
    const cfg = await r.json();
    applyCfg(cfg);
  }catch(e){
    // ignore
  }
}

['c_enabled','c_fan_enabled','c_fan_mode','c_fan_color',
 'n_enabled','n_stop','n_secs','n_mode','n_color',
 'p_n_enabled','p_n_fan_enabled','p_n_mode','p_n_color','p_n_ext_enabled','p_n_ext_mode','p_n_ext_color',
 'p_g_enabled','p_g_fan_enabled','p_g_mode','p_g_color','p_g_ext_enabled','p_g_ext_mode','p_g_ext_color']
.forEach(id=>{
  const el = $(id);
  el.addEventListener('change', scheduleSave);
  el.addEventListener('input', scheduleSave);
});

async function tick(){
  try{
    const r = await fetch('/api/state');
    const s = await r.json();

    $('t_cpu').textContent = fTemp(s.temps.cpu);
    $('t_gpu').textContent = fTemp(s.temps.gpu);
    $('t_soc').textContent = fTemp(s.temps.soc);
    $('t_batt').textContent = fTemp(s.temps.batt);

    $('st_zone').textContent = (s.zone && s.zone.name) ? s.zone.name : '?';
    $('st_reduce').textContent = ((s.zone && s.zone.reduce_percent) ? s.zone.reduce_percent : 0) + '%';
    $('st_screen').textContent = s.screen_on ? 'ON' : 'OFF';
    const chg = s.charging || {};
    $('st_chg').textContent = (chg.hw ? 'ON' : 'OFF') + (chg.enabled ? '' : ' (disabled)');
    $('st_game').textContent = s.game_mode ? 'ON' : 'OFF';
    $('st_idle').textContent = s.idle_mode ? 'ON' : 'OFF';
    $('st_ledprof').textContent = s.led_profile || '?';
    $('badge_profile').textContent = s.active_profile || '?';

    const fanSt = (s.leds && s.leds.fan_last_applied) ? s.leds.fan_last_applied : null;
    let fan = 'off';
    if(fanSt){ fan = fanSt.mode + ':' + fanSt.color; }
    $('st_fan').textContent = fan;

    let ext = 'off';
    // Notification override (OUT)
    if(s.leds && s.leds.external && s.leds.external.active){
      const st = s.leds.external.setting;
      const left = s.leds.external.ends_in_sec;
      ext = st ? (st.mode+':'+st.color) : 'active';
      if(left!==null && left!==undefined) ext += ' ('+left+'s)';
    } else {
      const baseExt = (s.leds && s.leds.base_external_last_applied) ? s.leds.base_external_last_applied : null;
      if(baseExt){ ext = baseExt.mode + ':' + baseExt.color; }
    }
    $('st_ext').textContent = ext;
    $('st_mem').textContent = (s.mem && s.mem.VmRSS_kb ? s.mem.VmRSS_kb : 0) + ' kB';

    if(s.last_config_error){ $('cfg_err').textContent = s.last_config_error; }
    else $('cfg_err').textContent = '';

    if(s.config_rev !== lastCfgRev){
      lastCfgRev = s.config_rev;
      await refreshCfg();
    }
  }catch(e){
    // ignore
  }
}

tick();
refreshCfg();

let tickTimer = null;
function startTicker(){
  if(tickTimer) clearInterval(tickTimer);
  const ms = document.hidden ? 5000 : 1200;
  tickTimer = setInterval(tick, ms);
}
document.addEventListener('visibilitychange', startTicker);
startTicker();

// initial page
setPage('status');
updateExtColorsByMode();
</script>

</body>
</html>"#;

    html.replace("#STYLE#", STYLE)
}

#[derive(Debug, Deserialize)]
struct UiSavePayload {
    charging: UiCharging,
    notifications: NotificationsConfig,
    profiles: UiProfiles,
}

#[derive(Debug, Deserialize)]
struct UiCharging {
    enabled: bool,
    #[serde(default)]
    fan_enabled: bool,
    #[serde(default)]
    fan_led: FanLedSetting,
}

#[derive(Debug, Deserialize)]
struct UiProfiles {
    normal: UiProfile,
    gaming: UiProfile,
}

#[derive(Debug, Deserialize)]
struct UiProfile {
    enabled: bool,
    #[serde(default)]
    fan_enabled: bool,
    #[serde(default)]
    fan_led: FanLedSetting,

    #[serde(default)]
    ext_enabled: bool,
    #[serde(default)]
    external_led: crate::user_config::ExternalLedSetting,
}

fn upsert_named_profile(
    cfg: &mut UserConfig,
    name: &str,
    kind: ProfileType,
    enabled: bool,
    fan: Option<FanLedSetting>,
    ext: Option<crate::user_config::ExternalLedSetting>,
) {
    if let Some(p) = cfg
        .profiles
        .iter_mut()
        .find(|p| p.name.eq_ignore_ascii_case(name) || p.profile_type == kind)
    {
        p.enabled = enabled;
        p.fan_led = fan;
        p.external_led = ext;
        // Keep name stable for UI.
        p.name = name.to_string();
        p.profile_type = kind;
        return;
    }

    let mut p = match kind {
        ProfileType::Normal => ProfileConfig::normal_default(),
        ProfileType::Gaming => ProfileConfig::gaming_default(),
        _ => ProfileConfig::normal_default(),
    };
    p.name = name.to_string();
    p.profile_type = kind;
    p.enabled = enabled;
    p.fan_led = fan;
    p.external_led = ext;
    cfg.profiles.push(p);
}

fn handle_api_save(shared: &Arc<RwLock<SharedState>>, cfg_path: &Path, body: &[u8]) -> Result<(), String> {
    let payload: UiSavePayload = serde_json::from_slice(body).map_err(|e| e.to_string())?;
    let mut cfg = { shared.read().unwrap().config.clone() };

    cfg.charging.enabled = payload.charging.enabled;
    cfg.charging.fan_led = if payload.charging.fan_enabled {
        Some(payload.charging.fan_led)
    } else {
        None
    };
    cfg.notifications = payload.notifications;

    upsert_named_profile(
        &mut cfg,
        "Normal",
        ProfileType::Normal,
        payload.profiles.normal.enabled,
        if payload.profiles.normal.fan_enabled { Some(payload.profiles.normal.fan_led) } else { None },
        if payload.profiles.normal.ext_enabled { Some(payload.profiles.normal.external_led) } else { None },
    );
    upsert_named_profile(
        &mut cfg,
        "Gaming",
        ProfileType::Gaming,
        payload.profiles.gaming.enabled,
        if payload.profiles.gaming.fan_enabled { Some(payload.profiles.gaming.fan_led) } else { None },
        if payload.profiles.gaming.ext_enabled { Some(payload.profiles.gaming.external_led) } else { None },
    );

    config_watch::apply_and_persist(shared, cfg_path, cfg)
}

pub fn spawn(shared: Arc<RwLock<SharedState>>, _leds: Arc<crate::leds::Leds>, cfg_path: PathBuf) {
    thread::spawn(move || {
        let server = match Server::http(BIND_ADDR) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("WEB: failed to bind {}: {}", BIND_ADDR, e);
                return;
            }
        };
        println!("WEB: http://{}", BIND_ADDR);

        for mut req in server.incoming_requests() {
            let url = req.url().to_string();
            let method = req.method().clone();

            // API-only mode: everything except /api/* returns an empty 404.
            if !url.starts_with("/api/") {
                let _ = req.respond(empty_404());
                continue;
            }

            // Require token-based auth for ALL API endpoints.
            if !is_authorized(&req, &shared) {
                let _ = req.respond(empty_404());
                continue;
            }

            let body = if matches!(method, Method::Post) {
                read_body(&mut req)
            } else {
                Vec::new()
            };

            let resp = match (method, url.as_str()) {
                (Method::Get, "/api/state") => ok_json(build_state_json(&shared)),

                // Read current effective config.
                (Method::Get, "/api/config") => {
                    let cfg = { shared.read().unwrap().config.clone() };
                    ok_json(serde_json::to_value(cfg).unwrap_or_else(|_| json!({})))
                }

                // Save (UI/app) config changes.
                (Method::Post, "/api/save") => match handle_api_save(&shared, &cfg_path, &body) {
                    Ok(_) => Response::from_string("ok"),
                    Err(e) => bad(400, &e),
                },

                _ => empty_404(),
            };

            let _ = req.respond(resp);
        }
    });
}
