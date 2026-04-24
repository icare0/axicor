## 2026-04-24 - Python fnv1a_32 hashing performance bottleneck
**Learning:** Python iterative algorithms on strings/bytes (like FNV-1a calculation inside a `for` loop) represent a significant bottleneck when called frequently in tight loops or initialization logic.
**Action:** Always memoize computationally expensive pure functions using `@functools.lru_cache(maxsize=1024)` to achieve massive speedups (e.g. 15x) when processing repetitive input parameters like hashing static strings (e.g., zone/matrix names).
