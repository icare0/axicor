import timeit
import functools

def fnv1a_32_original(data: bytes) -> int:
    hash_val = 0x811c9dc5
    for b in data:
        hash_val ^= b
        hash_val = (hash_val * 0x01000193) & 0xFFFFFFFF
    return hash_val

@functools.lru_cache(maxsize=1024)
def fnv1a_32_cached(data: bytes) -> int:
    hash_val = 0x811c9dc5
    for b in data:
        hash_val ^= b
        hash_val = (hash_val * 0x01000193) & 0xFFFFFFFF
    return hash_val

data = b"SensoryCortex"

# Warm up cache
fnv1a_32_cached(data)

setup = "from __main__ import fnv1a_32_original, fnv1a_32_cached, data"

t_orig = timeit.timeit("fnv1a_32_original(data)", setup=setup, number=1_000_000)
t_cached = timeit.timeit("fnv1a_32_cached(data)", setup=setup, number=1_000_000)

print(f"Original: {t_orig:.4f}s")
print(f"Cached: {t_cached:.4f}s")
print(f"Speedup: {t_orig / t_cached:.2f}x")
