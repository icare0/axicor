# Axicor: CartPole Architecture & Troubleshooting

This document summarizes critical architectural patterns and troubleshooting steps identified during the high-resolution CartPole agent implementation.

## Common Problems & Solutions

### 1. FATAL DMA BUFFER OVERFLOW
- **Symptom**: `axicor-node` panics during the Day Phase with an offset/buffer size error.
- **Cause**: Mismatch in `BATCH_SIZE` between `build_brain.py` and `agent.py`. The node allocates GPU/SHM buffers based on the brain's batch size, but the agent sends more ticks than expected.
- **Solution**: Ensure `BATCH_SIZE` is identical in both files.

### 2. Output batch exceeds UDP MTU
- **Symptom**: `axicor-orchestrator` panics with `Output batch exceeds UDP MTU`.
- **Cause**: The total size of a single output matrix (width * height * BATCH_SIZE) exceeds the hard UDP limit of **65,507 bytes**.
- **Example**: `160x80 * 20 = 256,000 bytes` (Panic). `40x40 * 20 = 32,000 bytes` (Safe).
- **Solution**: Reduce output resolution or batch size to stay well under the 65KB limit.

### 3. Neural Silence (0 Spikes in mid/deep zones)
- **Symptom**: L4 is spiking (receiving sensory input), but L2/3, L5, and L6 show 0 spikes.
- **Cause**: In the Axicor `BrainBuilder`, any matrix used as a source for `builder.connect()` **must** be explicitly declared via `add_output()` in the source zone's IO configuration. If missing from `io.toml`, the baker cannot determine the mapping dimensions and fails to grow the ghost axons.
- **Solution**: Always call `zone.add_output("matrix_name", ...)` even for internal-only ghost connections.

### 4. ValueError: buffer is smaller than requested size
- **Symptom**: Python agent crashes when calling `PwmDecoder.decode_from`.
- **Cause**: The agent receives a small internal-only UDP packet (e.g. `to_l23` sync packet) instead of the large `motor_out` matrix.
- **Solution**: Use `Matrix Hash` filtering in `AxicorMultiClient`. Pass `expected_rx_hash=fnv1a_32(b"motor_out")` to the `step()` function to ignore technical packets.

## Best Practices
- **Synchronize Ticks**: Use `BATCH_SIZE = 20` for stable, high-frequency HFT cycles.
- **Loop the Column**: Ensure feedback paths (`L6 -> L4`, `L2/3 -> L1`) are established to maintain neural dynamics.
- **Cleanup**: Use `scripts/clean_checkpoints.py` regularly to avoid accumulation of temporary simulation files.
