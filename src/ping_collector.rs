//! Background latency-measurement thread.
//!
//! Sends ICMP echo requests to 1.1.1.1 (Cloudflare DNS) every 5 seconds using
//! the Windows IcmpSendEcho API (no admin rights required).  Writes a rolling
//! [`PingSnapshot`] into the shared `Arc<RwLock<_>>`.

use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

use crate::models::PingSnapshot;

// 1.1.1.1 — same u32 in any byte order since all octets are identical.
const TARGET_IP: u32 = 0x0101_0101;
const TIMEOUT_MS: u32 = 3_000;
pub const INTERVAL: Duration = Duration::from_secs(5);
const SAMPLES: usize = 12; // 1 minute of history at 5 s cadence

pub fn start(snapshot: Arc<RwLock<PingSnapshot>>) {
    thread::Builder::new()
        .name("ping".into())
        .spawn(move || run(snapshot))
        .expect("failed to spawn ping thread");
}

fn run(snapshot: Arc<RwLock<PingSnapshot>>) {
    #[cfg(windows)]
    {
        use windows::Win32::NetworkManagement::IpHelper::IcmpCreateFile;
        let handle = match unsafe { IcmpCreateFile() } {
            Ok(h) if !h.is_invalid() => h,
            _ => return,
        };
        run_loop(snapshot, handle);
    }

    #[cfg(not(windows))]
    {
        // No ICMP support on non-Windows; thread stays alive but never updates.
        loop {
            thread::sleep(Duration::from_secs(60));
        }
    }
}

#[cfg(windows)]
fn run_loop(
    snapshot: Arc<RwLock<PingSnapshot>>,
    handle: windows::Win32::Foundation::HANDLE,
) {
    let mut history: VecDeque<Option<u32>> = VecDeque::with_capacity(SAMPLES);

    loop {
        let tick = Instant::now();

        let latency = unsafe { ping_icmp(handle) };

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
unsafe fn ping_icmp(handle: windows::Win32::Foundation::HANDLE) -> Option<u32> {
    use windows::Win32::NetworkManagement::IpHelper::{IcmpSendEcho, ICMP_ECHO_REPLY};

    let request = [0u8; 32];
    // Buffer must hold at least sizeof(ICMP_ECHO_REPLY) + request size + 8.
    let mut buf = [0u8; 256];

    let count = IcmpSendEcho(
        handle,
        TARGET_IP,
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
    }
}
