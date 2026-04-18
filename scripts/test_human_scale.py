import os
import sys
import math
from pathlib import Path

# Add SDK path
sys.path.append(os.path.abspath("genesis-client"))
from genesis.builder import BrainBuilder

def test_80b_estimation():
    builder = BrainBuilder(project_name="HumanScale_80B", output_dir="Genesis-Models/80B")
    
    # 1 maximum density shard (2047x2047x63) holds ~264 million neurons.
    # 80,000,000,000 / 263,991,561 ≈ 303 zones.
    print("⏳ Designing a connectome for 80 billion neurons...")
    
    for i in range(303):
        zone = builder.add_zone(f"Neocortex_Block_{i}", 2047, 2047, 63)
        zone.add_layer("GrayMatter", height_pct=1.0, density=1.0)
        
    print("\n" + builder.dry_run_stats())

if __name__ == "__main__":
    test_80b_estimation()
