#!/usr/bin/env python3
import os
import sqlite3
import pandas as pd
from pathlib import Path
from allensdk.core.cell_types_cache import CellTypesCache

# [DOD] Strict path resolution
BASE_DIR = Path(__file__).parent
RAW_DATA_DIR = BASE_DIR / "raw_data"
ALLEN_DIR = RAW_DATA_DIR / "allen_cell_types"
DB_PATH = RAW_DATA_DIR / "library_index.db"

def init_db():
    """Initializes the SQLite schema for the local biological cache."""
    RAW_DATA_DIR.mkdir(parents=True, exist_ok=True)
    ALLEN_DIR.mkdir(parents=True, exist_ok=True)
    
    conn = sqlite3.connect(DB_PATH)
    c = conn.cursor()
    # [DOD] Flat tabular layout. No ORM overhead.
    c.execute('''
        CREATE TABLE IF NOT EXISTS allen_ephys (
            specimen_id INTEGER PRIMARY KEY,
            structure_area_abbrev TEXT,
            dendrite_type TEXT,
            is_inhibitory BOOLEAN,
            v_rest_mv REAL,
            v_thresh_mv REAL,
            tau_ms REAL,
            rheobase_pa REAL,
            avg_firing_rate_hz REAL,
            has_morphology BOOLEAN
        )
    ''')
    conn.commit()
    return conn

def run_ingestion():
    print("[*] Starting Biological Data Ingestion (Allen Cell Types)...")
    conn = init_db()
    
    # Initialize Allen SDK Cache
    ctc = CellTypesCache(manifest_file=str(ALLEN_DIR / "manifest.json"))
    
    # Download metadata for all cells
    print("[*] Fetching cell metadata...")
    cells = ctc.get_cells()
    cells_df = pd.DataFrame(cells)
    
    # Download electrophysiology features
    print("[*] Fetching ephys features...")
    ephys_features = ctc.get_ephys_features()
    ephys_df = pd.DataFrame(ephys_features)
    
    # Join on specimen_id
    merged_df = pd.merge(cells_df, ephys_df, left_on='id', right_on='specimen_id')
    
    # Filter out cells missing critical data
    critical_columns = ['vrest', 'threshold_v_long_square', 'tau', 'threshold_i_long_square', 'avg_isi']
    merged_df = merged_df.dropna(subset=critical_columns)
    
    inserted_count = 0
    c = conn.cursor()
    
    print("[*] Inserting into SQLite cache...")
    for _, row in merged_df.iterrows():
        specimen_id = int(row['specimen_id'])
        structure = str(row.get('structure_area_abbrev', 'UNKNOWN'))
        dendrite_type = str(row.get('dendrite_type', 'UNKNOWN'))
        
        # Biological invariant: spiny = Excitatory (Glu), aspiny = Inhibitory (GABA)
        is_inhibitory = (dendrite_type == 'aspiny')
        
        v_rest_mv = float(row['vrest'])
        v_thresh_mv = float(row['threshold_v_long_square'])
        tau_ms = float(row['tau']) * 1000.0 # Convert seconds to ms
        rheobase_pa = float(row['threshold_i_long_square'])
        
        # Inter-spike interval to Frequency (Hz)
        avg_isi = float(row['avg_isi'])
        avg_firing_rate_hz = 1.0 / avg_isi if avg_isi > 0 else 0.0
        
        has_morphology = bool(row.get('has_reconstruction', False))
        
        c.execute('''
            INSERT OR REPLACE INTO allen_ephys 
            (specimen_id, structure_area_abbrev, dendrite_type, is_inhibitory, 
             v_rest_mv, v_thresh_mv, tau_ms, rheobase_pa, avg_firing_rate_hz, has_morphology)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ''', (specimen_id, structure, dendrite_type, is_inhibitory, 
              v_rest_mv, v_thresh_mv, tau_ms, rheobase_pa, avg_firing_rate_hz, has_morphology))
        
        inserted_count += 1

    conn.commit()
    conn.close()
    print(f"[OK] Ingestion complete. {inserted_count} viable cells cached in {DB_PATH}")

if __name__ == "__main__":
    run_ingestion()
