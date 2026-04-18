# Helper script for flygym inspection
#!/usr/bin/env python3
import numpy as np
from flygym.mujoco import NeuroMechFly

def inspect():
    print("🪰 Initializing NeuroMechFly for inspection...")
    env = NeuroMechFly()
    obs_space = env.observation_space
    act_space = env.action_space

    print("\n" + "="*50)
    print("OBSERVATIONS (RETINA INPUTS AND SENSORS)")
    print("="*50)
    total_obs = 0
    for k, v in obs_space.items():
        size = int(np.prod(v.shape))
        total_obs += size
        print(f"[{k}] Size: {size}, Shape: {v.shape}")
        # Output real limits to avoid division by zero during normalization
        _min = v.low.flatten()
        _max = v.high.flatten()
        print(f"    Min limits: {_min[:4]} ... {_min[-4:]}")
        print(f"    Max limits: {_max[:4]} ... {_max[-4:]}")
        
        # Search for potential dead zones (where min == max)
        dead_zones = np.where(_min == _max)
        if len(dead_zones) > 0:
            print(f"    ⚠️ DEAD ZONES FOUND at indices: {dead_zones}")

    print(f"\n[TOTAL SENSORS]: {total_obs} float variables")

    print("\n" + "="*50)
    print("ACTUATORS (MOTOR CORTEX OUTPUTS)")
    print("="*50)
    for k, v in act_space.items():
        size = int(np.prod(v.shape))
        print(f"[{k}] Size: {size}, Shape: {v.shape}")
        _min = v.low.flatten()
        _max = v.high.flatten()
        print(f"    Min limits: {_min[:4]} ...")
        print(f"    Max limits: {_max[:4]} ...")

if __name__ == "__main__":
    inspect()
