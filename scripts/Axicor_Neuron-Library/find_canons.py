import sqlite3
from pathlib import Path

db_path = Path('w:/Workspace/axicor/scripts/Axicor_Neuron-Library/raw_data/library_index.db')
conn = sqlite3.connect(db_path)
c = conn.cursor()

def get_specimen(query):
    c.execute(query)
    return c.fetchone()

# Selection Criteria
canons = {
    "Integrator": "SELECT specimen_id, structure_area_abbrev FROM allen_ephys WHERE dendrite_type='spiny' AND is_inhibitory=0 LIMIT 1",
    "Fast-Spiking": "SELECT specimen_id, structure_area_abbrev FROM allen_ephys WHERE dendrite_type='aspiny' AND is_inhibitory=1 ORDER BY avg_firing_rate_hz DESC LIMIT 1",
    "Pacemaker": "SELECT specimen_id, structure_area_abbrev FROM allen_ephys WHERE avg_firing_rate_hz > 5 LIMIT 1", # Lowered threshold if data is sparse
    "Relay": "SELECT specimen_id, structure_area_abbrev FROM allen_ephys WHERE structure_area_abbrev LIKE '%TH%' OR structure_area_abbrev LIKE '%LG%' LIMIT 1",
    "Martinotti": "SELECT specimen_id, structure_area_abbrev FROM allen_ephys WHERE dendrite_type='aspiny' AND structure_area_abbrev='VISp' LIMIT 1 OFFSET 2"
}

results = {}
for name, q in canons.items():
    results[name] = get_specimen(q)

for name, res in results.items():
    print(f"{name}: {res}")

conn.close()
