//! Tests for the coverage PC recording module.

use crate::coverage;
use std::collections::HashSet;

// Each test must reset to avoid cross-test contamination
// since coverage uses a global static.

#[test]
fn test_record_and_snapshot() {
    coverage::reset();

    coverage::record_pc(100);
    coverage::record_pc(200);
    coverage::record_pc(300);

    let snap = coverage::snapshot_pcs();
    assert_eq!(snap.len(), 3);
    assert!(snap.contains(&100));
    assert!(snap.contains(&200));
    assert!(snap.contains(&300));

    // Snapshot doesn't clear
    assert_eq!(coverage::count(), 3);
    coverage::reset();
}

#[test]
fn test_take_clears_state() {
    coverage::reset();

    coverage::record_pc(1);
    coverage::record_pc(2);
    assert_eq!(coverage::count(), 2);

    let taken = coverage::take_recorded_pcs();
    assert_eq!(taken.len(), 2);

    // After take, state is cleared
    assert_eq!(coverage::count(), 0);
    let empty = coverage::snapshot_pcs();
    assert!(empty.is_empty());
    coverage::reset();
}

#[test]
fn test_reset_clears_all() {
    coverage::reset();

    coverage::record_pc(42);
    assert_eq!(coverage::count(), 1);

    coverage::reset();
    assert_eq!(coverage::count(), 0);
    assert!(coverage::snapshot_pcs().is_empty());
}

#[test]
fn test_duplicate_pcs_deduplicated() {
    coverage::reset();

    for _ in 0..100 {
        coverage::record_pc(999);
    }

    assert_eq!(coverage::count(), 1);
    let snap = coverage::snapshot_pcs();
    assert_eq!(snap, HashSet::from([999]));
    coverage::reset();
}

#[test]
fn test_empty_snapshot_returns_empty_set() {
    coverage::reset();
    let snap = coverage::snapshot_pcs();
    assert!(snap.is_empty());
}

#[test]
fn test_empty_take_returns_empty_set() {
    coverage::reset();
    let taken = coverage::take_recorded_pcs();
    assert!(taken.is_empty());
}

#[test]
fn test_large_pc_values() {
    coverage::reset();

    coverage::record_pc(0);
    coverage::record_pc(u64::MAX);
    coverage::record_pc(u64::MAX / 2);

    assert_eq!(coverage::count(), 3);
    let snap = coverage::snapshot_pcs();
    assert!(snap.contains(&0u64));
    assert!(snap.contains(&u64::MAX));
    assert!(snap.contains(&(u64::MAX / 2)));
    coverage::reset();
}

#[test]
fn test_record_after_take_starts_fresh() {
    coverage::reset();

    coverage::record_pc(1);
    coverage::record_pc(2);
    let _ = coverage::take_recorded_pcs();

    coverage::record_pc(3);
    assert_eq!(coverage::count(), 1);
    let snap = coverage::snapshot_pcs();
    assert_eq!(snap, HashSet::from([3]));
    coverage::reset();
}
