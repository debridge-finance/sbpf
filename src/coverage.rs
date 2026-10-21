//! Global PC recording for coverage analysis.
//!
//! Records every PC executed by the SBF VM interpreter.
//! Uses a global mutex (not thread-local) because the VM interpreter
//! runs in a different thread than the test harness.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

#[derive(Default)]
struct CoverageState {
    pc_counts: HashMap<u64, u64>,
    stack_counts: HashMap<String, u64>,
    current_stack: Vec<u64>,
}

static COVERAGE: Mutex<Option<CoverageState>> = Mutex::new(None);

fn coverage() -> std::sync::MutexGuard<'static, Option<CoverageState>> {
    COVERAGE.lock().unwrap()
}

/// Record a PC value from the VM interpreter loop.
#[inline(always)]
pub fn record_pc(pc: u64) {
    let mut guard = coverage();
    let state = guard.get_or_insert_with(CoverageState::default);
    *state.pc_counts.entry(pc).or_insert(0) += 1;

    let mut stack = state.current_stack.clone();
    if stack.last().copied() != Some(pc) {
        stack.push(pc);
    }
    let key = stack
        .iter()
        .map(u64::to_string)
        .collect::<Vec<_>>()
        .join(";");
    *state.stack_counts.entry(key).or_insert(0) += 1;
}

/// Push a BPF-to-BPF call target onto the current guest stack.
#[inline(always)]
pub fn push_stack_frame(pc: u64) {
    coverage()
        .get_or_insert_with(CoverageState::default)
        .current_stack
        .push(pc);
}

/// Pop a BPF-to-BPF call frame from the current guest stack.
#[inline(always)]
pub fn pop_stack_frame() {
    let mut guard = coverage();
    if let Some(state) = guard.as_mut() {
        state.current_stack.pop();
    }
}

/// Take all recorded PCs and reset.
pub fn take_recorded_pcs() -> HashSet<u64> {
    take_pc_counts().into_keys().collect()
}

/// Get a snapshot without clearing.
pub fn snapshot_pcs() -> HashSet<u64> {
    snapshot_pc_counts().into_keys().collect()
}

/// Take all recorded PC hit counts and clear only the PC counts.
pub fn take_pc_counts() -> HashMap<u64, u64> {
    coverage()
        .as_mut()
        .map(|state| std::mem::take(&mut state.pc_counts))
        .unwrap_or_default()
}

/// Get a PC hit-count snapshot without clearing.
pub fn snapshot_pc_counts() -> HashMap<u64, u64> {
    coverage()
        .as_ref()
        .map(|state| state.pc_counts.clone())
        .unwrap_or_default()
}

/// Take all recorded stack hit counts and clear only the stack counts.
pub fn take_stack_counts() -> HashMap<String, u64> {
    coverage()
        .as_mut()
        .map(|state| std::mem::take(&mut state.stack_counts))
        .unwrap_or_default()
}

/// Get a stack hit-count snapshot without clearing.
pub fn snapshot_stack_counts() -> HashMap<String, u64> {
    coverage()
        .as_ref()
        .map(|state| state.stack_counts.clone())
        .unwrap_or_default()
}

/// Clear all recorded PCs.
pub fn reset() {
    *coverage() = None;
}

/// Get the count of recorded PCs.
pub fn count() -> usize {
    coverage()
        .as_ref()
        .map(|state| state.pc_counts.len())
        .unwrap_or(0)
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
    fn test_duplicate_pcs_counted() {
        reset();

        for _ in 0..100 {
            record_pc(999);
        }

        let counts = snapshot_pc_counts();
        assert_eq!(counts.get(&999), Some(&100));
        reset();
    }

    #[test]
    fn test_stack_counts_record_leaf_pc() {
        reset();

        push_stack_frame(10);
        record_pc(20);
        record_pc(20);

        let stacks = snapshot_stack_counts();
        assert_eq!(stacks.get("10;20"), Some(&2));
        reset();
    }

    #[test]
    fn test_stack_push_pop() {
        reset();

        push_stack_frame(10);
        push_stack_frame(20);
        pop_stack_frame();
        record_pc(30);

        let stacks = snapshot_stack_counts();
        assert_eq!(stacks.get("10;30"), Some(&1));
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

    #[test]
    fn test_take_pc_counts_clears_only_pcs() {
        reset();

        record_pc(1);
        record_pc(1);
        assert_eq!(take_pc_counts().get(&1), Some(&2));
        assert_eq!(count(), 0);
        assert_eq!(snapshot_stack_counts().get("1"), Some(&2));
        reset();
    }
}
