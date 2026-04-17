/// All signal.rs tests (§1.3 Active Tail).
/// Included via: `#[path = "test_signal.rs"] mod test_signal;` in signal.rs
use super::*;
use crate::constants::{AXON_SENTINEL, V_SEG};

#[test]
fn sentinel_is_never_active() {
    assert!(!is_segment_active(AXON_SENTINEL, 0, 10));
    assert!(!is_segment_active(AXON_SENTINEL, 100, 1));
}

#[test]
fn segment_in_tail_is_active() {
    let head = 10u32;
    assert!(is_segment_active(head, 8, 3));
    assert!(is_segment_active(head, 9, 3));
    assert!(is_segment_active(head, 10, 3)); // dist=0 < 3 → active
}

#[test]
fn segment_outside_tail_is_inactive() {
    let head = 10u32;
    assert!(!is_segment_active(head, 7, 3)); // dist=3, not < 3
    assert!(!is_segment_active(head, 0, 3)); // too far
}

#[test]
fn initial_head_is_not_sentinel() {
    let head = initial_axon_head(10);
    assert_ne!(head, AXON_SENTINEL);
    assert_ne!(head, 0);
}


/// initial_axon_head starts such that head reaches AXON_SENTINEL after length ticks.
/// In intermediate ticks (before SENTINEL) a segment in the tail must be active.
#[test]
fn axon_fires_after_length_ticks() {
    let length = 5u32;
    let propagation = 3u32;
    let mut head = initial_axon_head(length);

    // In length - propagation ticks, head hasn't "reached" SENTINEL yet,
    // but has shifted enough for segment to fall into the tail.
    // At tick length - 1, head is V_SEG away from SENTINEL.
    for _ in 0..(length - propagation) {
        head = head.wrapping_add(V_SEG);
    }

    // Now dist to initial segment < propagation → it must be active
    let starting_segment = AXON_SENTINEL.wrapping_sub(length * V_SEG);
    assert!(
        is_segment_active(head, starting_segment, propagation),
        "segment should be in active tail mid-flight. head={head:#010x}, seg={starting_segment:#010x}"
    );
}

/// Active Tail with width P covers exactly P segments.
#[test]
fn tail_width_equals_propagation_length() {
    let head = 100u32;
    let p = 7u32;

    let active_count = (0u32..200)
        .filter(|&s| is_segment_active(head, s, p))
        .count();

    assert_eq!(active_count, p as usize,
        "Active tail must cover exactly propagation_length={p} segments, got {active_count}");
}

/// Determinism: two calls with same arguments yield same result.
#[test]
fn is_segment_active_is_pure() {
    let head = 42u32;
    let seg = 40u32;
    let p = 3u32;
    assert_eq!(is_segment_active(head, seg, p), is_segment_active(head, seg, p));
}

/// Edge case: propagation_length = 0 → no segment is ever active.
#[test]
fn zero_propagation_length_never_active() {
    let head = 100u32;
    for seg in 95..=105 {
        assert!(!is_segment_active(head, seg, 0),
            "propagation=0 must never be active, seg={seg}");
    }
}

/// sentinel is always inactive regardless of propagation_length.
#[test]
fn sentinel_always_dead() {
    for p in [0u32, 1, 10, u32::MAX] {
        assert!(!is_segment_active(AXON_SENTINEL, 0, p),
            "AXON_SENTINEL must be inactive for propagation={p}");
    }
}
