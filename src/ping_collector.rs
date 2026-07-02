//! Background latency-measurement thread.
//!
//! Sends ICMP echo requests to a user-configurable target (default 1.1.1.1) every
//! 5 seconds using the Windows IcmpSendEcho API (no admin rights required).
//! Writes a rolling [`PingSnapshot`] into the shared `Arc<RwLock<_>>`.

use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

use crate::models::PingSnapshot;

const TIMEOUT_MS: u32 = 3_000;
pub const INTERVAL: Duration = Duration::from_secs(5);
const SAMPLES: usize = 12; // 1 minute of history at 5 s cadence

pub fn start(snapshot: Arc<RwLock<PingSnapshot>>, target: Arc<RwLock<String>>) {
    thread::Builder::new()
        .name("ping".into())
        .spawn(move || run(snapshot, target))
        .expect("failed to spawn ping thread");
}

fn run(snapshot: Arc<RwLock<PingSnapshot>>, target: Arc<RwLock<String>>) {
    #[cfg(windows)]
    {
        use windows::Win32::NetworkManagement::IpHelper::IcmpCreateFile;
        let handle = match unsafe { IcmpCreateFile() } {
            Ok(h) if !h.is_invalid() => h,
            _ => {
                log::warn!("IcmpCreateFile failed — ping card disabled");
                if let Ok(mut g) = snapshot.write() {
                    g.unavailable = true;
                }
                return;
            }
        };
        run_loop(snapshot, handle, target);
    }

    #[cfg(not(windows))]
    {
        // No ICMP support on non-Windows; thread stays alive but never updates.
        let _ = target;
        loop {
            thread::sleep(Duration::from_secs(60));
        }
    }
}

/// Resolve a hostname or dotted-decimal IPv4 string to a u32 in the byte order
/// that IcmpSendEcho expects (native-endian, first octet at byte 0 in memory).
fn resolve_ipv4(host: &str) -> Option<u32> {
    use std::net::{IpAddr, ToSocketAddrs};
    let h = host.trim();
    if let Ok(ip) = h.parse::<std::net::Ipv4Addr>() {
        return Some(u32::from_le_bytes(ip.octets()));
    }
    // DNS resolution — append :0 for the SocketAddr parser
    format!("{h}:0").to_socket_addrs().ok()?
        .find(|a| a.is_ipv4())
        .and_then(|a| if let IpAddr::V4(v4) = a.ip() {
            Some(u32::from_le_bytes(v4.octets()))
        } else {
            None
        })
}

#[cfg(windows)]
fn run_loop(
    snapshot: Arc<RwLock<PingSnapshot>>,
    handle: windows::Win32::Foundation::HANDLE,
    target: Arc<RwLock<String>>,
) {
    let mut history: VecDeque<Option<u32>> = VecDeque::with_capacity(SAMPLES);
    let mut last_target = String::new();
    let mut cached_ip: Option<u32> = None;

    loop {
        let tick = Instant::now();

        let current_target = target.read().map(|g| g.clone()).unwrap_or_else(|_| "1.1.1.1".into());

        // Clear history when the user changes the ping target so stale data doesn't mix.
        if current_target != last_target {
            history.clear();
            cached_ip = None;
            last_target = current_target.clone();
        }

        // Resolve once and cache — a blocking DNS lookup every cycle is wasteful.
        if cached_ip.is_none() {
            cached_ip = resolve_ipv4(&current_target);
        }
        let latency = cached_ip.and_then(|ip| unsafe { ping_icmp(handle, ip) });
        // Re-resolve after a failure — the host may have changed address.
        if latency.is_none() {
            cached_ip = None;
        }

        if history.len() >= SAMPLES {
            history.pop_front();
        }
        history.push_back(latency);

        let snap = compute(&history);
        if let Ok(mut g) = snapshot.write() {
            *g = snap;
        }

        let elapsed = tick.elapsed();
        if elapsed < INTERVAL {
            thread::sleep(INTERVAL - elapsed);
        }
    }
}

#[cfg(windows)]
unsafe fn ping_icmp(handle: windows::Win32::Foundation::HANDLE, target_ip: u32) -> Option<u32> {
    use windows::Win32::NetworkManagement::IpHelper::{IcmpSendEcho, ICMP_ECHO_REPLY};

    let request = [0u8; 32];
    // Buffer must hold at least sizeof(ICMP_ECHO_REPLY) + request size + 8.
    let mut buf = [0u8; 256];

    let count = IcmpSendEcho(
        handle,
        target_ip,
        request.as_ptr() as *const _,
        request.len() as u16,
        None,
        buf.as_mut_ptr() as *mut _,
        buf.len() as u32,
        TIMEOUT_MS,
    );

    if count == 0 {
        return None;
    }

    let reply = &*(buf.as_ptr() as *const ICMP_ECHO_REPLY);
    // Status 0 = IP_SUCCESS
    if reply.Status != 0 {
        return None;
    }
    Some(reply.RoundTripTime)
}

fn compute(history: &VecDeque<Option<u32>>) -> PingSnapshot {
    if history.is_empty() {
        return PingSnapshot::default();
    }

    let successes: Vec<u32> = history.iter().filter_map(|x| *x).collect();
    let total = history.len();
    let losses = history.iter().filter(|x| x.is_none()).count();
    let loss_pct = losses as f32 / total as f32 * 100.0;

    let avg_ms = if successes.is_empty() {
        0.0
    } else {
        successes.iter().sum::<u32>() as f32 / successes.len() as f32
    };

    let jitter_ms = if successes.len() < 2 {
        0.0
    } else {
        successes
            .iter()
            .map(|&v| (v as f32 - avg_ms).abs())
            .sum::<f32>()
            / successes.len() as f32
    };

    PingSnapshot {
        latency_ms: history.back().copied().flatten(),
        avg_ms,
        jitter_ms,
        loss_pct,
        sample_count: total,
        unavailable: false,
    }
}
