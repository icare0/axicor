/// The Train Model (1.1) tests with various `v_seg` and `propagation_length`.
/// Ensures no "gaps" during head jumps if `prop_len >= v_seg`.
use super::*;
use crate::constants::{AXON_SENTINEL, V_SEG};

#[test]
fn test_no_gap_v_seg_1() {
    let v_seg = 1;
    let prop_len = 5;

    // Simulate 50 ticks
    let start_head = 100u32;
    let mut current_head = start_head;

    let mut covered_segments = vec![];

    for _ in 0..50 {
        current_head = current_head.wrapping_add(v_seg);

        // In each tick, some segments are active. Remember them.
        // Active tail: (head - prop_len ..= head]  (if < prop_len)
        for seg in 0..200 {
            if is_segment_active(current_head, seg, prop_len) {
                covered_segments.push(seg);
            }
        }
    }

    // Sort and dedup
    covered_segments.sort_unstable();
    covered_segments.dedup();

    // Check that there are no "gaps" (unvisited segments) in the range from start_head to start_head + 50*v_seg
    // All segments that were within this range should be in covered_segments.
    let min_expected = start_head + v_seg;
    let max_expected = start_head + 50 * v_seg - prop_len + 1;

    for seg in min_expected..=max_expected {
        assert!(
            covered_segments.contains(&seg),
            "Segment {} was missed! v_seg={}, prop={}",
            seg,
            v_seg,
            prop_len
        );
    }
}

#[test]
fn test_no_gap_v_seg_large() {
    let v_seg = 5;
    let prop_len = 5; // prop >= v_seg, should be no gaps

    let start_head = 100u32;
    let mut current_head = start_head;

    let mut covered_segments = vec![];

    for _ in 0..20 {
        current_head = current_head.wrapping_add(v_seg);
        for seg in 0..300 {
            if is_segment_active(current_head, seg, prop_len) {
                covered_segments.push(seg);
            }
        }
    }

    covered_segments.sort_unstable();
    covered_segments.dedup();

    let min_expected = start_head + v_seg;
    let max_expected = start_head + 20 * v_seg - prop_len + 1;

    for seg in min_expected..=max_expected {
        assert!(
            covered_segments.contains(&seg),
            "Segment {} was missed! v_seg={}, prop={}",
            seg,
            v_seg,
            prop_len
        );
    }
}

#[test]
fn test_gap_when_prop_lt_v_seg() {
    let v_seg = 5;
    let prop_len = 3; // prop < v_seg, MUST have gaps

    let start_head = 100u32;
    let mut current_head = start_head;

    let mut covered_segments = vec![];

    for _ in 0..20 {
        current_head = current_head.wrapping_add(v_seg);
        for seg in 0..300 {
            if is_segment_active(current_head, seg, prop_len) {
                covered_segments.push(seg);
            }
        }
    }

    covered_segments.sort_unstable();
    covered_segments.dedup();

    // Jumps: 100 -> 105, 105 -> 110.
    // If prop=3, then tails are [103..105], [108..110].
    // Segments 101, 102, 106, 107 are skipped!
    assert!(
        !covered_segments.contains(&106),
        "Segment 106 should be MISSED, but it was covered!"
    );
    assert!(
        !covered_segments.contains(&107),
        "Segment 107 should be MISSED, but it was covered!"
    );
}

#[test]
fn test_roundtrip_fire_to_overlap() {
    let length = 10u32;
    // use v_seg from constant for this test
    let mut head = initial_axon_head(length);
    let prop_len = 3;

    for _ in 0..length {
        head = head.wrapping_add(V_SEG);
    }

    // On tick `length`, head should be near 0 for segment 0
    let _start_segment = AXON_SENTINEL.wrapping_sub(length * V_SEG);

    // If we shifted initial_head by length * V_SEG, we end up at start_segment + length * V_SEG = AXON_SENTINEL
    // Oh wow, the initial segment is AXON_SENTINEL - length * V_SEG. The end is AXON_SENTINEL.
    // So when head == AXON_SENTINEL, the signal is "dead".
    // But let's check 1 tick EARLIER:

    let mut before_end_head = initial_axon_head(length);
    for _ in 0..(length - 1) {
        before_end_head = before_end_head.wrapping_add(V_SEG);
    }

    let last_segment = AXON_SENTINEL.wrapping_sub(V_SEG);
    assert!(
        is_segment_active(before_end_head, last_segment, prop_len),
        "Last segment must be active just before sentinel!"
    );
}
