import sqlite3
from pathlib import Path

BASE_DIR = Path(__file__).parent
db_path = BASE_DIR / "raw_data" / "library_index.db"
conn = sqlite3.connect(db_path)
c = conn.cursor()

def get_specimen(query, params=()):
    c.execute(query, params)
    return c.fetchone()

# Selection Criteria
canons = {
    "Integrator": ("SELECT specimen_id, structure_area_abbrev FROM allen_ephys WHERE dendrite_type=? AND is_inhibitory=? LIMIT 1", ('spiny', 0)),
    "Fast-Spiking": ("SELECT specimen_id, structure_area_abbrev FROM allen_ephys WHERE dendrite_type=? AND is_inhibitory=? ORDER BY avg_firing_rate_hz DESC LIMIT 1", ('aspiny', 1)),
    "Pacemaker": ("SELECT specimen_id, structure_area_abbrev FROM allen_ephys WHERE avg_firing_rate_hz > ? LIMIT 1", (5,)), # Lowered threshold if data is sparse
    "Relay": ("SELECT specimen_id, structure_area_abbrev FROM allen_ephys WHERE structure_area_abbrev LIKE ? OR structure_area_abbrev LIKE ? LIMIT 1", ('%TH%', '%LG%')),
    "Martinotti": ("SELECT specimen_id, structure_area_abbrev FROM allen_ephys WHERE dendrite_type=? AND structure_area_abbrev=? LIMIT 1 OFFSET 2", ('aspiny', 'VISp'))
}

results = {}
for name, (q, params) in canons.items():
    results[name] = get_specimen(q, params)

for name, res in results.items():
    print(f"{name}: {res}")

conn.close()
