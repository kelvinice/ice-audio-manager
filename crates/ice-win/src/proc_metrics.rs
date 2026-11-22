//! Per-process memory (`PrivateUsage` / Private Bytes, `PrivateWorkingSetSize`) and CPU sampling.

use core::ffi::c_void;
use std::mem::size_of;
use std::time::Duration;

use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE};
use windows::Win32::System::ProcessStatus::{
    GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX2,
};
use windows::Win32::System::Threading::{
    GetCurrentProcessId, GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};

/// Memory and CPU counter snapshot for a single process.
#[derive(Debug, Clone)]
pub struct ProcessSample {
    pub pid: u32,
    pub kind: String,
    pub private_bytes: u64,
    pub private_working_set: u64,
    pub working_set: u64,
    pub cpu_100ns: u64,
}

/// Aggregated memory and CPU snapshot across the host process and app-owned WebView2 child processes.
#[derive(Debug, Clone)]
pub struct TreeSnapshot {
    pub processes: Vec<ProcessSample>,
    pub total_private_bytes: u64,
    pub total_private_ws: u64,
    pub total_working_set: u64,
    pub total_cpu_100ns: u64,
}

struct OwnedHandle(HANDLE);
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            // SAFETY: handle was opened by `OpenProcess` and is closed exactly once.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
}

/// Returns the current process ID.
#[must_use]
pub fn current_pid() -> u32 {
    // SAFETY: plain Win32 query returning current process ID.
    unsafe { GetCurrentProcessId() }
}

/// Returns the number of logical CPUs available on this system.
#[must_use]
pub fn logical_cpu_count() -> u32 {
    std::thread::available_parallelism().map_or(1, |n| u32::try_from(n.get()).unwrap_or(u32::MAX)).max(1)
}

/// Samples Private Bytes, Private Working Set, Total Working Set, and CPU times across `pids`.
#[must_use]
pub fn sample_pids(pids: &[(u32, String)]) -> TreeSnapshot {
    let mut processes = Vec::with_capacity(pids.len());
    let mut total_private_bytes = 0u64;
    let mut total_private_ws = 0u64;
    let mut total_working_set = 0u64;
    let mut total_cpu_100ns = 0u64;

    for (pid, kind) in pids {
        if let Some(sample) = sample_one(*pid, kind) {
            total_private_bytes = total_private_bytes.saturating_add(sample.private_bytes);
            total_private_ws = total_private_ws.saturating_add(sample.private_working_set);
            total_working_set = total_working_set.saturating_add(sample.working_set);
            total_cpu_100ns = total_cpu_100ns.saturating_add(sample.cpu_100ns);
            processes.push(sample);
        }
    }

    TreeSnapshot { processes, total_private_bytes, total_private_ws, total_working_set, total_cpu_100ns }
}

/// Computes normalized system CPU percentage (`0.0..=100.0`) between two snapshots over `elapsed`.
#[must_use]
pub fn cpu_percent_between(before: &TreeSnapshot, after: &TreeSnapshot, elapsed: Duration) -> f64 {
    let elapsed_100ns = elapsed.as_secs_f64() * 10_000_000.0;
    if elapsed_100ns <= 0.0 {
        return 0.0;
    }
    let cpus = f64::from(logical_cpu_count());
    // Sum delta per matching PID so short-lived exiting child processes don't make total delta negative.
    let mut delta_100ns = 0u64;
    for after_p in &after.processes {
        let prev =
            before.processes.iter().find(|p| p.pid == after_p.pid && p.kind == after_p.kind).map_or(0, |p| p.cpu_100ns);
        delta_100ns = delta_100ns.saturating_add(after_p.cpu_100ns.saturating_sub(prev));
    }
    (delta_100ns as f64 / (elapsed_100ns * cpus)) * 100.0
}

fn sample_one(pid: u32, kind: &str) -> Option<ProcessSample> {
    // SAFETY: opens read-only limited query access to `pid`; wrapped in `OwnedHandle`.
    let handle = OwnedHandle(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()? });

    let mut counters = PROCESS_MEMORY_COUNTERS_EX2 {
        cb: u32::try_from(size_of::<PROCESS_MEMORY_COUNTERS_EX2>()).ok()?,
        ..Default::default()
    };
    // SAFETY: `PROCESS_MEMORY_COUNTERS_EX2` begins with `PROCESS_MEMORY_COUNTERS` and `cb` holds the exact struct size.
    unsafe {
        GetProcessMemoryInfo(handle.0, (&raw mut counters).cast::<PROCESS_MEMORY_COUNTERS>(), counters.cb).ok()?;
    }

    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: stack `FILETIME` pointers are valid for the synchronous call.
    unsafe {
        GetProcessTimes(handle.0, &raw mut creation, &raw mut exit, &raw mut kernel, &raw mut user).ok()?;
    }
    let cpu_100ns = filetime_to_u64(kernel).saturating_add(filetime_to_u64(user));

    let _ = &raw const counters as *const c_void;
    Some(ProcessSample {
        pid,
        kind: kind.to_string(),
        private_bytes: u64::try_from(counters.PrivateUsage).unwrap_or(0),
        private_working_set: u64::try_from(counters.PrivateWorkingSetSize).unwrap_or(0),
        working_set: u64::try_from(counters.WorkingSetSize).unwrap_or(0),
        cpu_100ns,
    })
}

const fn filetime_to_u64(ft: FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
}
