//! Global PC recording for coverage analysis.
//!
//! Records every PC executed by the SBF VM interpreter.
//! Uses a global mutex (not thread-local) because the VM interpreter
//! runs in a different thread than the test harness.

use std::collections::HashSet;
use std::sync::Mutex;

static RECORDED_PCS: Mutex<Option<HashSet<u64>>> = Mutex::new(None);

fn pcs() -> std::sync::MutexGuard<'static, Option<HashSet<u64>>> {
    RECORDED_PCS.lock().unwrap()
}

/// Record a PC value from the VM interpreter loop.
#[inline(always)]
pub fn record_pc(pc: u64) {
    let mut guard = pcs();
    guard.get_or_insert_with(HashSet::new).insert(pc);
}

/// Take all recorded PCs and reset.
pub fn take_recorded_pcs() -> HashSet<u64> {
    pcs().take().unwrap_or_default()
}

/// Get a snapshot without clearing.
pub fn snapshot_pcs() -> HashSet<u64> {
    pcs().as_ref().cloned().unwrap_or_default()
}

/// Clear all recorded PCs.
pub fn reset() {
    *pcs() = None;
}

/// Get the count of recorded PCs.
pub fn count() -> usize {
    pcs().as_ref().map(|s| s.len()).unwrap_or(0)
}

#[cfg(test)]
#[path = "coverage_tests.rs"]
mod tests;
