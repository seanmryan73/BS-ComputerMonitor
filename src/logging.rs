// Author  : Sean Ryan <seanmryan@gmail.com>
// Company : BagPipes

//! File logging per the vault's Rust-Desktop-Standards § Logging Standard.
//!
//! One file per launch at `%LOCALAPPDATA%\BSComputerMonitor\logs\log-<yyyyMMdd-HHmmss>.log`, newest
//! 20 kept. Local, not Roaming: settings roam with a profile, diagnostic logs should not. Until
//! 2026-10-09 this app logged only through a bare `env_logger` to stderr, which a
//! `windows_subsystem = "windows"` release build discards - so every `warn!` it wrote went nowhere.
//!
//! Lines carry local time of day, then elapsed since launch: `[14:03:22.418 +012:41.207 INFO ]`.
//! The first line after midnight is preceded by `date is now YYYY-MM-DD` (elapsed-only until
//! 2026-10-28, which a 20-hour session could not line up with Event Viewer). The date is in the
//! file name and in the banner, which is written straight to the file so it appears at any `RUST_LOG` level. The default
//! filter is `warn,bc_computer_monitor=info`: this always-running monitor logs lifecycle and
//! saved-state changes at `info` and nothing per sample, so `info` costs a few lines a session.
//! Until 2026-10-20 it was plain `warn`, which left every log holding only the banner.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime};

use chrono::Datelike;
use env_logger::{Builder, Env, Target};

/// Folder name under both `%APPDATA%` (settings) and `%LOCALAPPDATA%` (logs).
pub(crate) const APP_DIR: &str = "BSComputerMonitor";
const KEEP_LOGS: usize = 20;
/// `warn` for dependencies (wgpu/winit chatter); `info` for this crate, which logs lifecycle and
/// saved-state changes there and nothing per sample (vault § Logging Standard, monitors).
const DEFAULT_FILTER: &str = "warn,bc_computer_monitor=info";
/// Present while a session runs; finding it at start means the last one never reached `on_exit`.
const SESSION_MARKER: &str = "session.running";

/// How often the collector re-stamps the marker, so its modified time says when the session was
/// last alive rather than when it started.
pub const HEARTBEAT: Duration = Duration::from_secs(60);
/// A shutdown event this close to the last heartbeat explains the missing `on_exit`. Windows logs
/// 1074 seconds before it ends apps and 6006 seconds after; the heartbeat adds up to a minute.
const SHUTDOWN_SLACK: Duration = Duration::from_secs(120);

/// Call once, after the single-instance check so a second copy can't trip it. When the previous
/// session died without `on_exit`, a background thread asks the System event log why: a Windows
/// shutdown runs no exit handler, so until 2026-10-28 every power-off logged a false "crash" (3 of
/// 3 warnings). A native fault (GPU driver, ETW, WGC) leaves no panic and no log line, so this is
/// its only trace.
pub fn mark_session_start() {
    if let Some(dir) = log_dir() {
        if let Some(last_alive) = mark_session_start_in(&dir) {
            std::thread::spawn(move || log_previous_end(last_alive));
        }
    }
}

/// Returns the previous session's last heartbeat when it left its marker behind.
fn mark_session_start_in(dir: &Path) -> Option<SystemTime> {
    let marker = dir.join(SESSION_MARKER);
    let last_alive = marker.exists().then(|| {
        fs::metadata(&marker).and_then(|m| m.modified()).unwrap_or_else(|e| {
            log::warn!("could not read the time of {}: {e}", marker.display());
            SystemTime::now()
        })
    });
    if let Err(e) = fs::write(&marker, std::process::id().to_string()) {
        log::warn!("could not write {}: {e}; unclean exits won't be detected", marker.display());
    }
    last_alive
}

/// Called every [`HEARTBEAT`] from the collector thread, which keeps running while the window is
/// hidden in the tray.
pub fn touch_session_marker() {
    static WARNED: AtomicBool = AtomicBool::new(false);
    let Some(marker) = log_dir().map(|d| d.join(SESSION_MARKER)) else { return };
    if let Err(e) = fs::write(&marker, std::process::id().to_string()) {
        // Once: this repeats every minute and the cause won't change.
        if !WARNED.swap(true, Ordering::Relaxed) {
            log::warn!("could not refresh {}: {e}; a shutdown may read as a crash", marker.display());
        }
    }
}

#[derive(Debug, PartialEq)]
enum PreviousEnd {
    Shutdown,
    PowerLoss,
    Unexplained,
}

fn log_previous_end(last_alive: SystemTime) {
    let at = chrono::DateTime::<chrono::Local>::from(last_alive).format("%Y-%m-%d %H:%M:%S");
    match classify_previous_end(last_alive) {
        Ok(PreviousEnd::Shutdown) => log::info!(
            "previous session (last alive {at}) ended with a Windows shutdown or restart \
             (System event 1074/6006)"
        ),
        Ok(PreviousEnd::PowerLoss) => log::warn!(
            "previous session (last alive {at}) ended when Windows stopped without shutting down \
             (System event 41/6008: power loss, a hang or a bugcheck)"
        ),
        Ok(PreviousEnd::Unexplained) => log::warn!(
            "previous session (last alive {at}) did not exit cleanly and Windows did not shut down \
             then: a crash or a kill. A native fault is in Event Viewer > Application, Event ID 1000, \
             naming bc-computer-monitor.exe"
        ),
        Err(e) => log::warn!(
            "previous session (last alive {at}) did not exit cleanly; the System event log could \
             not be read ({e}), so a crash, a kill and a shutdown look the same"
        ),
    }
}

fn classify_previous_end(last_alive: SystemTime) -> Result<PreviousEnd, String> {
    let from = last_alive.checked_sub(SHUTDOWN_SLACK).unwrap_or(last_alive);
    let to = last_alive + SHUTDOWN_SLACK;
    if system_log_has(&event_query(&[1074, 6006], from, Some(to)))? {
        return Ok(PreviousEnd::Shutdown);
    }
    // 41 and 6008 are written at the next boot, so they have no upper bound.
    if system_log_has(&event_query(&[41, 6008], last_alive, None))? {
        return Ok(PreviousEnd::PowerLoss);
    }
    Ok(PreviousEnd::Unexplained)
}

/// Event Log XPath for any of `ids` created in `[from, to]`.
fn event_query(ids: &[u32], from: SystemTime, to: Option<SystemTime>) -> String {
    let utc = |t: SystemTime| {
        chrono::DateTime::<chrono::Utc>::from(t).format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
    };
    let ids = ids.iter().map(|id| format!("EventID={id}")).collect::<Vec<_>>().join(" or ");
    let mut time = format!("@SystemTime>='{}'", utc(from));
    if let Some(to) = to {
        time.push_str(&format!(" and @SystemTime<='{}'", utc(to)));
    }
    format!("*[System[({ids}) and TimeCreated[{time}]]]")
}

#[cfg(windows)]
fn system_log_has(query: &str) -> Result<bool, String> {
    use windows::core::HSTRING;
    use windows::Win32::System::EventLog::{EvtClose, EvtNext, EvtQuery, EvtQueryChannelPath, EVT_HANDLE};

    // SAFETY: both handles come from the calls above them and are closed exactly once.
    unsafe {
        let results = EvtQuery(
            EVT_HANDLE(0),
            &HSTRING::from("System"),
            &HSTRING::from(query),
            EvtQueryChannelPath.0,
        )
        .map_err(|e| e.message())?;
        let mut event = [0isize; 1];
        let mut returned = 0u32;
        // An empty result is ERROR_NO_MORE_ITEMS, so any error here means "none found".
        let found = EvtNext(results, &mut event, 1000, 0, &mut returned).is_ok() && returned > 0;
        if found {
            let _ = EvtClose(EVT_HANDLE(event[0]));
        }
        let _ = EvtClose(results);
        Ok(found)
    }
}

#[cfg(not(windows))]
fn system_log_has(_query: &str) -> Result<bool, String> {
    Err("no Windows event log on this platform".into())
}

/// First thing in `on_exit`.
pub fn clear_session_marker() {
    if let Some(dir) = log_dir() {
        clear_session_marker_in(&dir);
    }
}

fn clear_session_marker_in(dir: &Path) {
    let marker = dir.join(SESSION_MARKER);
    if let Err(e) = fs::remove_file(&marker) {
        if e.kind() != std::io::ErrorKind::NotFound {
            log::warn!("could not remove {}: {e}", marker.display());
        }
    }
}

static LAUNCH: OnceLock<Instant> = OnceLock::new();

/// `None` when `LOCALAPPDATA` is unset: `init` then logs to stderr. Never `.`, which for a
/// double-clicked EXE is the install folder.
pub(crate) fn log_dir() -> Option<PathBuf> {
    log_dir_under(std::env::var_os("LOCALAPPDATA"))
}

fn log_dir_under(local_app_data: Option<std::ffi::OsString>) -> Option<PathBuf> {
    local_app_data.map(|base| PathBuf::from(base).join(APP_DIR).join("logs"))
}

/// `mmm:ss.mmm`, fixed width so the column sorts as text; minutes do not wrap at 60.
fn format_elapsed(ms: u128) -> String {
    format!("{:03}:{:02}.{:03}", ms / 60_000, ms / 1000 % 60, ms % 1000)
}

/// Deletes all but the newest `keep` `.log` files in `dir` (names sort chronologically).
fn prune_old_logs(dir: &Path, keep: usize) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "log"))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries.iter().take(entries.len().saturating_sub(keep)) {
        let _ = fs::remove_file(e.path()); // best effort; a leftover log is harmless
    }
}

/// `[14:03:22.418 +012:41.207 INFO ]`: time of day lines up with Event Viewer, elapsed answers
/// "how long".
fn format_stamp(time_of_day: chrono::NaiveTime, elapsed_ms: u128, level: &str) -> String {
    format!("[{} +{} {level:<5}]", time_of_day.format("%H:%M:%S%.3f"), format_elapsed(elapsed_ms))
}

/// Day (days since CE) of the last line written; `date is now` goes out when it changes.
static LAST_DAY: AtomicI32 = AtomicI32::new(i32::MIN);

fn builder() -> Builder {
    let mut builder = Builder::from_env(Env::default().default_filter_or(DEFAULT_FILTER));
    builder.format(|buf, record| {
        let ms = LAUNCH.get_or_init(Instant::now).elapsed().as_millis();
        let now = chrono::Local::now();
        let level = record.level().to_string();
        let day = now.date_naive().num_days_from_ce();
        let previous = LAST_DAY.swap(day, Ordering::Relaxed);
        if previous != i32::MIN && previous != day {
            writeln!(buf, "{} date is now {}", format_stamp(now.time(), ms, "INFO"), now.format("%Y-%m-%d"))?;
        }
        writeln!(buf, "{} {}", format_stamp(now.time(), ms, &level), record.args())
    });
    builder
}

/// Starts file logging. Falls back to stderr rather than blocking startup: logging is a diagnostic
/// aid, not load-bearing. Must run before anything logs. Returns the log file when one was opened.
pub fn init() -> Option<PathBuf> {
    let _ = LAUNCH.set(Instant::now());
    let now = chrono::Local::now();
    LAST_DAY.store(now.date_naive().num_days_from_ce(), Ordering::Relaxed);
    if let Some(dir) = log_dir() {
        if fs::create_dir_all(&dir).is_ok() {
            // Prune to KEEP-1 so the file created below brings the total to KEEP.
            prune_old_logs(&dir, KEEP_LOGS - 1);
            let path = dir.join(format!("log-{}.log", now.format("%Y%m%d-%H%M%S")));
            if let Ok(mut file) = File::create(&path) {
                // pid and exe path tell a stale running copy from the build just made.
                let exe = std::env::current_exe()
                    .map_or_else(|e| format!("<unknown: {e}>"), |p| p.display().to_string());
                let _ = writeln!(
                    file,
                    "started at {} v{} pid {} exe {exe}",
                    now.format("%Y-%m-%d %H:%M:%S %z"),
                    env!("CARGO_PKG_VERSION"),
                    std::process::id()
                );
                builder().target(Target::Pipe(Box::new(file))).init();
                return Some(path);
            }
        }
    }
    builder().init();
    None
}

/// A panic goes to stderr, which the release build discards, so `log` never sees it without this.
/// Chains to the previous hook to keep debug backtraces, and flushes because the process is about
/// to die.
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info.location().map_or_else(
            || "unknown location".to_owned(),
            |l| format!("{}:{}", l.file(), l.line()),
        );
        let message = info.payload_as_str().unwrap_or("<non-string panic payload>");
        // force_capture: the default hook prints a backtrace only with RUST_BACKTRACE set, and only
        // to stderr, which a GUI build discards (Rust-Desktop-Standards.md § Panics need their own hook).
        let backtrace = std::backtrace::Backtrace::force_capture();
        log::error!("PANIC at {location}: {message}\n{backtrace}");
        log::logger().flush();
        previous(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_localappdata_means_no_log_dir_not_the_working_directory() {
        assert_eq!(log_dir_under(None), None);
        assert_eq!(
            log_dir_under(Some(r"C:\Users\u\AppData\Local".into())),
            Some(PathBuf::from(r"C:\Users\u\AppData\Local\BSComputerMonitor\logs"))
        );
    }

    #[test]
    fn elapsed_stamps_are_fixed_width_and_do_not_wrap_at_an_hour() {
        assert_eq!(format_elapsed(0), "000:00.000");
        assert_eq!(format_elapsed(61_234), "001:01.234");
        assert_eq!(format_elapsed(3_600_000), "060:00.000");
    }

    #[test]
    fn stamps_carry_time_of_day_then_elapsed() {
        let t = chrono::NaiveTime::from_hms_milli_opt(14, 3, 22, 418).unwrap();
        assert_eq!(format_stamp(t, 761_207, "INFO"), "[14:03:22.418 +012:41.207 INFO ]");
        assert_eq!(format_stamp(t, 0, "WARN"), "[14:03:22.418 +000:00.000 WARN ]");
    }

    #[test]
    fn shutdown_query_bounds_both_ends_and_power_loss_only_the_start() {
        let at = SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000);
        assert_eq!(
            event_query(&[1074, 6006], at, Some(at + Duration::from_secs(120))),
            "*[System[(EventID=1074 or EventID=6006) and TimeCreated[\
             @SystemTime>='2027-01-15T08:00:00.000Z' and @SystemTime<='2027-01-15T08:02:00.000Z']]]"
        );
        assert_eq!(
            event_query(&[41], at, None),
            "*[System[(EventID=41) and TimeCreated[@SystemTime>='2027-01-15T08:00:00.000Z']]]"
        );
    }

    /// Reads this machine's real System log; a bad XPath fails here rather than as a silent
    /// "unexplained" on the next unclean start.
    #[cfg(windows)]
    #[test]
    fn the_system_log_accepts_the_queries() {
        let long_ago = SystemTime::now() - Duration::from_secs(365 * 24 * 3600);
        assert_eq!(system_log_has(&event_query(&[6005, 6006], long_ago, None)), Ok(true));
        assert_eq!(classify_previous_end(SystemTime::now() + Duration::from_secs(3600)), Ok(PreviousEnd::Unexplained));
    }

    #[test]
    fn prune_keeps_the_newest_and_ignores_other_files() {
        let dir = std::env::temp_dir().join(format!("bscm-logtest-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        for name in [
            "log-20260101-000000.log",
            "log-20260102-000000.log",
            "log-20260103-000000.log",
            "notes.txt",
        ] {
            File::create(dir.join(name)).unwrap();
        }
        prune_old_logs(&dir, 2);
        let mut left: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(
            left,
            ["log-20260102-000000.log", "log-20260103-000000.log", "notes.txt"]
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_marker_left_behind_reports_an_unclean_exit() {
        let dir = std::env::temp_dir().join(format!("bscm-markertest-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        assert!(mark_session_start_in(&dir).is_none(), "first run is clean");
        assert!(mark_session_start_in(&dir).is_some(), "no on_exit in between");
        clear_session_marker_in(&dir);
        assert!(mark_session_start_in(&dir).is_none(), "on_exit cleared it");
        clear_session_marker_in(&dir);
        clear_session_marker_in(&dir); // already gone: no error
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_info_filter_names_this_crate() {
        // A crate rename would otherwise silently drop the filter back to warn.
        assert!(DEFAULT_FILTER.ends_with(&format!("{}=info", env!("CARGO_CRATE_NAME"))));
    }
}
