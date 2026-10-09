// Author  : Sean Ryan <seanmryan@gmail.com>
// Company : BagPipes
// Version : 2026.10.09

//! File logging per the vault's Rust-Desktop-Standards § Logging Standard.
//!
//! One file per launch at `%LOCALAPPDATA%\BSComputerMonitor\logs\log-<yyyyMMdd-HHmmss>.log`, newest
//! 20 kept. Local, not Roaming: settings roam with a profile, diagnostic logs should not. Until
//! 2026-10-09 this app logged only through a bare `env_logger` to stderr, which a
//! `windows_subsystem = "windows"` release build discards - so every `warn!` it wrote went nowhere.
//!
//! Lines carry elapsed-since-launch (`mmm:ss.mmm`); the wall clock is in the file name and in the
//! banner, which is written straight to the file so it appears at any `RUST_LOG` level. The default
//! level is `warn`: this is an always-running monitor, and `info` per sample would bury the file.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Instant;

use env_logger::{Builder, Env, Target};

const APP_DIR: &str = "BSComputerMonitor";
const KEEP_LOGS: usize = 20;

static LAUNCH: OnceLock<Instant> = OnceLock::new();

/// `None` when `LOCALAPPDATA` is unset: `init` then logs to stderr. Never `.`, which for a
/// double-clicked EXE is the install folder.
fn log_dir() -> Option<PathBuf> {
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

fn builder() -> Builder {
    let mut builder = Builder::from_env(Env::default().default_filter_or("warn"));
    builder.format(|buf, record| {
        let ms = LAUNCH.get_or_init(Instant::now).elapsed().as_millis();
        writeln!(buf, "[{} {:<5}] {}", format_elapsed(ms), record.level(), record.args())
    });
    builder
}

/// Starts file logging. Falls back to stderr rather than blocking startup: logging is a diagnostic
/// aid, not load-bearing. Must run before anything logs. Returns the log file when one was opened.
pub fn init() -> Option<PathBuf> {
    let _ = LAUNCH.set(Instant::now());
    let now = chrono::Local::now();
    if let Some(dir) = log_dir() {
        if fs::create_dir_all(&dir).is_ok() {
            // Prune to KEEP-1 so the file created below brings the total to KEEP.
            prune_old_logs(&dir, KEEP_LOGS - 1);
            let path = dir.join(format!("log-{}.log", now.format("%Y%m%d-%H%M%S")));
            if let Ok(mut file) = File::create(&path) {
                let _ = writeln!(
                    file,
                    "started at {} v{}",
                    now.format("%Y-%m-%d %H:%M:%S %z"),
                    env!("CARGO_PKG_VERSION")
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
        log::error!("PANIC at {location}: {message}");
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
}
