#!/usr/bin/env python3
import time
import gc
import numpy as np
import cv2
import sys
import os

# Add SDK path
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "axicor-client")))
from axicor.retina import RetinaEncoder

def run_retina_stress_test():
    W, H = 256, 256
    B = 20  # sync_batch_ticks
    
    print(f" Initializing RetinaEncoder ({W}x{H}, Batch={B} ticks)...")
    retina = RetinaEncoder(width=W, height=H, batch_size=B)

    # 1. C-ABI Warp Alignment Check
    expected_bytes_per_tick = ((W * H + 31) // 32) * 4
    expected_total = expected_bytes_per_tick * B
    
    print(f" C-ABI Contract:")
    print(f"   - Pixels: {W * H}")
    print(f"   - Bytes per tick: {expected_bytes_per_tick} (32-bit aligned: {expected_bytes_per_tick % 4 == 0})")
    print(f"   - Payload size: {expected_total} bytes")
    
    assert retina.total_bytes == expected_total, f"FATAL: C-ABI Alignment broken! Expected {expected_total}, got {retina.total_bytes}"

    # 2. Network buffer emulation (including 20-byte ExternalIoHeader)
    tx_buffer = bytearray(20 + expected_total)
    tx_view = memoryview(tx_buffer)

    # 3. Mock frame preallocation (Zero-Garbage)
    mock_frame = np.zeros((H, W, 3), dtype=np.uint8)

    # Isolate Garbage Collector
    gc.collect()
    gc.disable()
    # Capture initial object count (Gen 0)
    start_gen0 = gc.get_count()[0]

    ITERATIONS = 10_000
    print(f"\n Starting Hot Loop for {ITERATIONS} frames...")
    
    start_t = time.perf_counter()
    for _ in range(ITERATIONS):
        # In-place frame mutation (camera matrix noise emulation) 
        # Performs zero heap allocations in Python
        cv2.randu(mock_frame, 0, 255)
        
        # Encoding
        retina.encode_into(mock_frame, tx_view[20:])

    elapsed = time.perf_counter() - start_t
    
    end_gen0 = gc.get_count()[0]
    gc.enable()

    fps = ITERATIONS / elapsed
    tps = fps * B
    # Bandwidth in Mbps
    bandwidth_mbps = (expected_total * 8 * fps) / (1024**2)

    print("\n" + "="*40)
    print(" PROFILING RESULTS")
    print("="*40)
    print(f" Execution time:     {elapsed:.3f} sec")
    print(f" Performance:        {fps:,.0f} FPS")
    print(f" Network equivalent:  {tps:,.0f} TPS (Ticks Per Second)")
    print(f" Data stream:        {bandwidth_mbps:.2f} Mbps")
    print(f" Objects in Gen 0:   {start_gen0} -> {end_gen0} (Delta: {end_gen0 - start_gen0})")
    
    if end_gen0 - start_gen0 > 10:
        print(f"[ERROR] FAILURE: Hidden allocation detected in Hot Loop! (Delta: {end_gen0 - start_gen0})")
        sys.exit(1)
    else:
        print("[OK] ZERO-GARBAGE INVARIANT CONFIRMED")

if __name__ == '__main__':
    run_retina_stress_test()
