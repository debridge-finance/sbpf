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
mod tests {
    use super::*;

    #[test]
    fn test_record_and_snapshot() {
        reset();

        record_pc(100);
        record_pc(200);
        record_pc(300);

        let snap = snapshot_pcs();
        assert_eq!(snap.len(), 3);
        assert!(snap.contains(&100));
        assert!(snap.contains(&200));
        assert!(snap.contains(&300));

        // Snapshot doesn't clear
        assert_eq!(count(), 3);
        reset();
    }

    #[test]
    fn test_take_clears_state() {
        reset();

        record_pc(1);
        record_pc(2);
        assert_eq!(count(), 2);

        let taken = take_recorded_pcs();
        assert_eq!(taken.len(), 2);

        // After take, state is cleared
        assert_eq!(count(), 0);
        let empty = snapshot_pcs();
        assert!(empty.is_empty());
        reset();
    }

    #[test]
    fn test_reset_clears_all() {
        reset();

        record_pc(42);
        assert_eq!(count(), 1);

        reset();
        assert_eq!(count(), 0);
        assert!(snapshot_pcs().is_empty());
    }

    #[test]
    fn test_duplicate_pcs_deduplicated() {
        reset();

        for _ in 0..100 {
            record_pc(999);
        }

        assert_eq!(count(), 1);
        let snap = snapshot_pcs();
        assert_eq!(snap, HashSet::from([999]));
        reset();
    }

    #[test]
    fn test_empty_snapshot_returns_empty_set() {
        reset();
        let snap = snapshot_pcs();
        assert!(snap.is_empty());
    }

    #[test]
    fn test_empty_take_returns_empty_set() {
        reset();
        let taken = take_recorded_pcs();
        assert!(taken.is_empty());
    }

    #[test]
    fn test_large_pc_values() {
        reset();

        record_pc(0);
        record_pc(u64::MAX);
        record_pc(u64::MAX / 2);

        assert_eq!(count(), 3);
        let snap = snapshot_pcs();
        assert!(snap.contains(&0u64));
        assert!(snap.contains(&u64::MAX));
        assert!(snap.contains(&(u64::MAX / 2)));
        reset();
    }

    #[test]
    fn test_record_after_take_starts_fresh() {
        reset();

        record_pc(1);
        record_pc(2);
        let _ = take_recorded_pcs();

        record_pc(3);
        assert_eq!(count(), 1);
        let snap = snapshot_pcs();
        assert_eq!(snap, HashSet::from([3]));
        reset();
    }
}
