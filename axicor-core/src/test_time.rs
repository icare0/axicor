/// Temporal metric tests (§1.4).
use super::*;
use crate::constants::TICK_DURATION_US;

#[test]
fn ms_to_ticks_basic() {
    // 5 ms / 0.1 ms = 50 ticks
    assert_eq!(ms_to_ticks(5.0), 50);
    // 1 ms = 10 ticks
    assert_eq!(ms_to_ticks(1.0), 10);
    // 0.1 ms = 1 tick
    assert_eq!(ms_to_ticks(0.1), 1);
}

#[test]
fn us_to_ticks_basic() {
    // 500 us / 100 us = 5 ticks
    assert_eq!(us_to_ticks(500), 5);
    // 100 us = 1 tick
    assert_eq!(us_to_ticks(TICK_DURATION_US), 1);
    // 0 us = 0 ticks
    assert_eq!(us_to_ticks(0), 0);
}

#[test]
fn ticks_to_ms_roundtrip() {
    let ms = 7.5f32;
    let ticks = ms_to_ticks(ms);
    let back = ticks_to_ms(ticks);
    assert!((back - ms).abs() < 0.01, "roundtrip: {ms} ms → {ticks} ticks → {back} ms");
}

#[test]
fn biological_refractory_example() {
    // Spec §1.4: 5 ms refractoriness = 50 ticks
    assert_eq!(ms_to_ticks(5.0), 50);
}
