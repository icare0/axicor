#!/usr/bin/env python3
import os
import toml
from pathlib import Path

# [DOD] Strict Paths
BASE_DIR = Path(__file__).parent
CATALOG_DIR = BASE_DIR / "Axicor_Neuron-Lib"

def run_synaptic_calibration():
    print("[*] Starting Zone C Calibration (Synaptic Weights & GSOP Plasticity)...")
    
    processed = 0
    for file_path in CATALOG_DIR.glob("*.toml"):
        # Skip manifest
        if file_path.name == "_manifest.json":
            continue
            
        with open(file_path, "r") as f:
            profile = toml.load(f)
            
        # Fix: neuron_type is a list
        nt = profile["neuron_type"][0]
        
        # 1. Scale Weights to penetrate biological thresholds
        # Cortical pyramidal cells need strong recurrents. We set base to 12mV.
        nt["initial_synapse_weight"] = 12000 
        
        # 2. GSOP (STDP) Plasticity parameters
        # Asymmetric Hebbian learning: Depression is slightly stronger than Potentiation for stability
        nt["gsop_potentiation"] = 400
        nt["gsop_depression"] = 450
        
        # 3. Structural limits
        nt["prune_threshold"] = 1000 # Sever connections dropping below 1mV
        
        with open(file_path, "w") as f:
            toml.dump(profile, f)
            
        processed += 1

    print(f"[OK] Zone C Calibration complete. Updated {processed} profiles.")

if __name__ == "__main__":
    run_synaptic_calibration()
