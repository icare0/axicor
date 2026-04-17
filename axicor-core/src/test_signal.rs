/// Все тесты signal.rs (§1.3 Active Tail).
/// Подключается через: `#[path = "test_signal.rs"] mod test_signal;` в signal.rs
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
    assert!(is_segment_active(head, 10, 3)); // dist=0 < 3 → входит
}

#[test]
fn segment_outside_tail_is_inactive() {
    let head = 10u32;
    assert!(!is_segment_active(head, 7, 3)); // dist=3, не < 3
    assert!(!is_segment_active(head, 0, 3)); // слишком далеко
}

#[test]
fn initial_head_is_not_sentinel() {
    let head = initial_axon_head(10);
    assert_ne!(head, AXON_SENTINEL);
    assert_ne!(head, 0);
}


/// initial_axon_head стартует так чтобы через length тиков голова достигла AXON_SENTINEL.
/// В промежуточных тиках (пока не SENTINEL) сегмент в хвосте должен быть активен.
#[test]
fn axon_fires_after_length_ticks() {
    let length = 5u32;
    let propagation = 3u32;
    let mut head = initial_axon_head(length);

    // За length - propagation тиков голова ещё не «добежала» до SENTINEL,
    // но уже достаточно сдвинулась чтобы сегмент попал в хвост.
    // На тике length - 1 голова находится на расстоянии V_SEG от SENTINEL.
    for _ in 0..(length - propagation) {
        head = head.wrapping_add(V_SEG);
    }

    // Сейчас dist до начального сегмента < propagation → он должен быть активен
    let starting_segment = AXON_SENTINEL.wrapping_sub(length * V_SEG);
    assert!(
        is_segment_active(head, starting_segment, propagation),
        "segment should be in active tail mid-flight. head={head:#010x}, seg={starting_segment:#010x}"
    );
}

/// Active Tail шириной P покрывает ровно P сегментов.
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

/// Детерминизм: два вызова с одинаковыми аргументами дают одинаковый результат.
#[test]
fn is_segment_active_is_pure() {
    let head = 42u32;
    let seg = 40u32;
    let p = 3u32;
    assert_eq!(is_segment_active(head, seg, p), is_segment_active(head, seg, p));
}

/// Граничный случай: propagation_length = 0 → никакой сегмент не активен.
#[test]
fn zero_propagation_length_never_active() {
    let head = 100u32;
    for seg in 95..=105 {
        assert!(!is_segment_active(head, seg, 0),
            "propagation=0 must never be active, seg={seg}");
    }
}

/// sentinel всегда неактивен независимо от propagation_length.
#[test]
fn sentinel_always_dead() {
    for p in [0u32, 1, 10, u32::MAX] {
        assert!(!is_segment_active(AXON_SENTINEL, 0, p),
            "AXON_SENTINEL must be inactive for propagation={p}");
    }
}
