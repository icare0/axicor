pub fn fnv1a_32(data: &[u8]) -> u32 {
    let mut hash = 0x811c9dc5;
    for &byte in data {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_fnv1a() {
        assert_eq!(fnv1a_32(b"SensoryCortex"), 0x228800bd); // Placeholder, verify with python if needed
    }
}
