## 2026-04-24 - Python fnv1a_32 hashing performance bottleneck
**Learning:** Python iterative algorithms on strings/bytes (like FNV-1a calculation inside a `for` loop) represent a significant bottleneck when called frequently in tight loops or initialization logic.
**Action:** Always memoize computationally expensive pure functions using `@functools.lru_cache(maxsize=1024)` to achieve massive speedups (e.g. 15x) when processing repetitive input parameters like hashing static strings (e.g., zone/matrix names).

## 2026-05-04 - Numpy packbits performance vs C-Backend Array multiply/sum
**Learning:** Manual bit-packing using `np.multiply` on a boolean view combined with `np.sum(..., axis=2)` (to replicate C-like bit packing without allocations) is much slower (~2x to ~10x) than using numpy's native `np.packbits`.
**Action:** Always prefer `np.packbits(..., axis=1, bitorder='little')` over manual boolean scaling and summation, while retaining zero-allocation performance by slicing into the pre-allocated view: `self._packed_buffer[:] = np.packbits(...)`.
