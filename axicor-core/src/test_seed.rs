/// Master Seed and wyhash tests (2).
use super::*;

#[test]
fn same_string_same_seed() {
    let s1 = MasterSeed::from_str("GENESIS_DAEMON_TEST");
    let s2 = MasterSeed::from_str("GENESIS_DAEMON_TEST");
    assert_eq!(s1, s2);
}

#[test]
fn different_strings_different_seeds() {
    let s1 = MasterSeed::from_str("GENESIS_DAEMON_TEST");
    let s2 = MasterSeed::from_str("GENESIS_DAEMON_TEST2");
    assert_ne!(s1, s2);
}

#[test]
fn messy_string_test() {
    // Random garbage, spaces, Chinese (and others) characters, emojis
    let messy = "   GENESIS   __ 2026      \n\t_!!   $#@%   ";
    let s = MasterSeed::from_str(messy);
    assert_ne!(s.raw(), 0, "Seed should not be 0 even for messy input");

    // Hash is stable
    let s2 = MasterSeed::from_str(messy);
    assert_eq!(s.raw(), s2.raw());
}

#[test]
fn empty_string_handled_safely() {
    let s = MasterSeed::from_str("");
    // wyhash handles empty string correctly
    let s2 = MasterSeed::from_str("");
    assert_eq!(s, s2);
}

#[test]
fn raw_not_equal_to_literal() {
    // Bug demonstration: old hardcoded "GENESIS" bytes is not "GENESIS" passed through wyhash
    let old_hardcode: u64 = 0x47454E455349530;
    let real_seed = MasterSeed::from_str("GENESIS");
    assert_ne!(
        old_hardcode,
        real_seed.raw(),
        "Hardcoded string bytes do not equal wyhash hash!"
    );
}

#[test]
fn random_f32_range() {
    for i in 0..10_000u32 {
        let f = random_f32(entity_seed(42, i));
        assert!(f >= 0.0 && f < 1.0, "random_f32 out of bounds: {f}");
    }
}

#[test]
fn test_avalanche_effect() {
    let master = 0x1234567890ABCDEF;
    let s1 = entity_seed(master, 10);
    let s2 = entity_seed(master, 11);

    assert_ne!(s1, s2, "Adjacent IDs must have different seeds");

    // Check that bits are thoroughly mixed (minimum 16 bits difference - heuristic)
    let diff_bits = (s1 ^ s2).count_ones();
    assert!(
        diff_bits >= 16,
        "Avalanche effect too weak: only {diff_bits} bits differ"
    );
}
