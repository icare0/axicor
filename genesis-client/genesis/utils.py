def fnv1a_32(data: bytes) -> int:
    """Детерминированный хэш (совпадает с реализацией на Rust)."""
    hash_val = 0x811c9dc5
    for b in data:
        hash_val ^= b
        hash_val = (hash_val * 0x01000193) & 0xFFFFFFFF
    return hash_val
