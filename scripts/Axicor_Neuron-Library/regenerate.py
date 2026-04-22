#!/usr/bin/env python3
import os
import math
import sqlite3
import toml
import json
import time
from pathlib import Path
from schema import generate_scaffold

# [DOD] Strict Paths
BASE_DIR = Path(__file__).parent
DB_PATH = BASE_DIR / "raw_data" / "library_index.db"
CATALOG_DIR = BASE_DIR.parent.parent / "Axicor_Neuron-Lib"

def calc_leak_shift(tau_ms: float) -> int:
    if tau_ms <= 0.1: return 1
    decay = math.exp(-0.1 / tau_ms)
    fraction_lost = 1.0 - decay
    if fraction_lost <= 0: return 16
    k = round(math.log2(1.0 / fraction_lost))
    return max(1, min(16, k))

def regenerate_catalog():
    print("[*] Starting Full Catalog Regeneration (Epic 10)...")
    CATALOG_DIR.mkdir(parents=True, exist_ok=True)
    
    conn = sqlite3.connect(DB_PATH)
    c = conn.cursor()
    c.execute("""
        SELECT specimen_id, structure_area_abbrev, is_inhibitory, 
               v_rest_mv, v_thresh_mv, tau_ms, avg_firing_rate_hz 
        FROM allen_ephys
    """)
    rows = c.fetchall()
    
    manifest = {
        "version": "1.0",
        "timestamp": int(time.time()),
        "total_profiles": 0,
        "profiles": []
    }
    
    processed = 0
    for row in rows:
        specimen_id, region, is_inh, v_rest, v_thr, tau, hz = row
        is_inhibitory = bool(is_inh)
        
        profile_name = f"{region}_{specimen_id}"
        profile = generate_scaffold(profile_name, is_inhibitory)
        # Fix: nt is the first element of the neuron_type list
        nt = profile["neuron_type"][0]
        
        nt["rest_potential"] = int(v_rest * 1000.0)
        nt["threshold"] = int(v_thr * 1000.0)
        nt["ahp_amplitude"] = 5000 
        
        shift = calc_leak_shift(tau)
        nt["leak_shift"] = shift
        nt["adaptive_leak_min_shift"] = max(1, shift - 2)
        
        if hz > 0.1:
            nt["spontaneous_firing_period_ticks"] = int(10000.0 / hz)
        else:
            nt["spontaneous_firing_period_ticks"] = 0
            
        out_path = CATALOG_DIR / f"{specimen_id}.toml"
        with open(out_path, "w") as f:
            toml.dump(profile, f)
            
        manifest["profiles"].append({
            "specimen_id": specimen_id,
            "region": region,
            "is_inhibitory": is_inhibitory,
            "file": f"{specimen_id}.toml"
        })
        processed += 1

    manifest["total_profiles"] = processed
    
    with open(CATALOG_DIR / "_manifest.json", "w") as f:
        json.dump(manifest, f, indent=2)

    conn.close()
    print(f"[OK] Regeneration complete. {processed} profiles written. Manifest saved.")

if __name__ == "__main__":
    regenerate_catalog()
