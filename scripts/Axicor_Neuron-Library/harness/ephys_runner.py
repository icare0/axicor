import os
import time
import struct
import mmap
import numpy as np
from pathlib import Path

# [DOD] C-ABI Constants
EPHYS_MAGIC = 0x45504859
MAX_TARGETS = 16
MAX_TICKS = 10000

def get_ephys_shm_path(zone_hash: int) -> str:
    filename = f"axicor_ephys_{zone_hash:08X}.shm"
    if os.name == 'nt':
        import tempfile
        return os.path.join(tempfile.gettempdir(), filename)
    return f"/dev/shm/{filename}"

class EphysHarness:
    def __init__(self, zone_hash: int):
        self.zone_hash = zone_hash
        self.shm_path = get_ephys_shm_path(zone_hash)
        
        if not os.path.exists(self.shm_path):
            raise FileNotFoundError(f"SHM not found: {self.shm_path}. Is axicor-node running?")
            
        self.file = open(self.shm_path, "r+b")
        self.mm = mmap.mmap(self.file.fileno(), 0)

    def run_ephys_protocol(self, target_tids: list[int], injection_uv: list[int], ticks: int = 1000) -> np.ndarray:
        count = len(target_tids)
        assert count <= MAX_TARGETS, "Exceeded MAX_TARGETS"
        assert ticks <= MAX_TICKS, "Exceeded MAX_TICKS"
        assert count == len(injection_uv), "TIDs and UVs length mismatch"

        # 1. Write Data (Zero-Copy packing)
        tids_arr = np.array(target_tids, dtype=np.uint32)
        uvs_arr = np.array(injection_uv, dtype=np.int32)
        
        self.mm[64:64 + (count * 4)] = tids_arr.tobytes()
        self.mm[128:128 + (count * 4)] = uvs_arr.tobytes()

        # 2. Fire Trigger (state = 1)
        struct.pack_into("<IIIII", self.mm, 0, EPHYS_MAGIC, 1, count, ticks, 0)

        # 3. Spin-wait for Node completion (state == 3)
        print(f"[*] Ephys Triggered. Waiting for GPU to process {ticks} ticks...")
        while True:
            state = struct.unpack_from("<I", self.mm, 4)[0]
            if state == 3:
                break
            time.sleep(0.01)

        # 4. Zero-Copy extraction of V(t) trace directly from mapped RAM
        trace_flat = np.ndarray(shape=(count * ticks,), dtype=np.int32, buffer=self.mm, offset=192)
        trace_2d = trace_flat.reshape((count, ticks)).copy() # Copy to release mmap lock
        
        # 5. Reset state
        struct.pack_into("<I", self.mm, 4, 0)
        
        return trace_2d

    def close(self):
        self.mm.close()
        self.file.close()

def test_smoke():
    # Хардкод хэша для теста (например, SensoryCortex)
    try:
        from axicor.utils import fnv1a_32
        zone_hash = fnv1a_32(b"SensoryCortex")
    except ImportError:
        def fnv1a_32_local(data: bytes) -> int:
            hash_val = 0x811C9DC5
            for b in data:
                hash_val ^= b
                hash_val = (hash_val * 0x01000193) & 0xFFFFFFFF
            return hash_val
        zone_hash = fnv1a_32_local(b"SensoryCortex")
    
    harness = EphysHarness(zone_hash)
    
    # Инъекция 100 mV (100000 uV) в нулевой нейрон на 1000 тиков
    print("Running Ephys Injection...")
    traces = harness.run_ephys_protocol(target_tids=[0], injection_uv=[100000], ticks=1000)
    
    # Экспорт в CSV для Опуса
    import csv
    out_file = "debug_v_trace_smoke.csv"
    with open(out_file, "w", newline='') as f:
        writer = csv.writer(f)
        writer.writerow(["Tick", "V_uV"])
        for tick, v in enumerate(traces[0]):
            writer.writerow([tick, v])
            
    print(f"[OK] Smoke test complete. Trace saved to {out_file}")
    harness.close()

if __name__ == "__main__":
    test_smoke()
