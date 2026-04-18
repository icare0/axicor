import numpy as np
import sys
def check_shard(name, path):
    try:
        data = np.fromfile(path, dtype=np.uint8)
        # [DOD FIX] 1166 bytes per neuron (14 soma + 128 * (4 targets + 4 weights + 1 timers))
        n = len(data) // 1166
        # Soma: 14*n, Targets: 128*n*4, Weights: 128*n*4
        off_w = 14 * n + 128 * n * 4
        # Read weights (i32)
        w = np.frombuffer(data[off_w : off_w + n * 128 * 4], dtype=np.int32)
        
        active = w[w != 0]
        if len(active) == 0:
            print(f"[{name}] All weights are ZERO")
            return
        exc_count = len(active[active > 0])
        inh_count = len(active[active < 0])
        
        # Biological target: 80% Exc / 20% Inh => 4:1
        actual_ratio = exc_count / inh_count if inh_count > 0 else float('inf')
        target_ratio = 4.0  # (80/20)
        deviation = actual_ratio / target_ratio
        
        status = "OK" if 0.5 <= deviation <= 2.0 else "I-DOMINANCE" if deviation < 0.5 else "E-DOMINANCE"
        
        # [DOD FIX] Shift Mass Domain back to Charge Domain
        print(f"[{name}] Stats:")
        print(f"  - Active Synapses: {len(active)} ({(len(active)/(n*128))*100:.2f}% density)")
        print(f"  - Avg Weight:      {np.mean(np.abs(active)) / 65536.0:.2f}")
        print(f"  - Max/Min:         {np.max(active) // 65536} / {np.min(active) // 65536}")
        print(f"  - Exc/Inh Ratio:   {exc_count} / {inh_count} ({actual_ratio:.4f}) | Target: {target_ratio:.2f} | {status}")
        print(f"  - Balance Health:  {deviation*100:.2f}% of target")
    except Exception as e:
        print(f"[{name}] Error: {e}")
check_shard("Cerebellum", "Genesis_Models/mouse_agent/baked/Cerebellum/checkpoint.state")
check_shard("Hippocampus", "Genesis_Models/mouse_agent/baked/Hippocampus/checkpoint.state")
check_shard("LGN_Thalamus", "Genesis_Models/mouse_agent/baked/LGN_Thalamus/checkpoint.state")
check_shard("Motor_Cortex", "Genesis_Models/mouse_agent/baked/Motor_Cortex/checkpoint.state")
check_shard("PFC_Cortex", "Genesis_Models/mouse_agent/baked/PFC_Cortex/checkpoint.state")
check_shard("Striatum", "Genesis_Models/mouse_agent/baked/Striatum/checkpoint.state")
check_shard("V1_Cortex", "Genesis_Models/mouse_agent/baked/V1_Cortex/checkpoint.state")
