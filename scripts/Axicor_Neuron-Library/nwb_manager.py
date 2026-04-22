#!/usr/bin/env python3
import os
from pathlib import Path
from allensdk.core.cell_types_cache import CellTypesCache

# [DOD] Strict path resolution
BASE_DIR = Path(__file__).parent
RAW_DATA_DIR = BASE_DIR / "raw_data"
ALLEN_DIR = RAW_DATA_DIR / "allen_cell_types"

class NwbManager:
    """
    Zero-Cost wrapper for Allen SDK. 
    Downloads NWB (HDF5) files and GLIF models strictly ON-DEMAND.
    """
    def __init__(self):
        ALLEN_DIR.mkdir(parents=True, exist_ok=True)
        self.ctc = CellTypesCache(manifest_file=str(ALLEN_DIR / "manifest.json"))

    def get_ephys_data(self, specimen_id: int):
        """Lazy fetching of NWB file. Network I/O occurs ONLY if file is missing."""
        # Returns an NwbDataSet object
        return self.ctc.get_ephys_data(specimen_id)

    def get_glif_models(self, specimen_id: int):
        """Fetches pre-computed Allen GLIF model parameters for baseline."""
        return self.ctc.get_glif_models(specimen_id=specimen_id)

if __name__ == "__main__":
    print("[OK] NWB Manager initialized. Ready for Lazy Loading during Phase B.")
