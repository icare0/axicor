## 2026-04-24 - Python fnv1a_32 hashing performance bottleneck
**Learning:** Python iterative algorithms on strings/bytes (like FNV-1a calculation inside a `for` loop) represent a significant bottleneck when called frequently in tight loops or initialization logic.
**Action:** Always memoize computationally expensive pure functions using `@functools.lru_cache(maxsize=1024)` to achieve massive speedups (e.g. 15x) when processing repetitive input parameters like hashing static strings (e.g., zone/matrix names).
## 2024-05-24 - Numpy Bit Packing Optimization
**Learning:** In hot loops within `axicor-client` needing zero-allocation operations, utilizing `np.dot` with `out=` avoids the intermediate array allocations inherent in chained `np.multiply` and `np.sum` operations, improving performance.
**Action:** When manually packing bits or doing operations that involve multiplying and then summing in Numpy, use `np.dot(bits_view, powers, out=packed_buffer)` to eliminate GC overhead and intermediate matrix allocations.
