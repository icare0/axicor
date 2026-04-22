import os
import glob
import re

for file_path in glob.glob("GNM-Library/**/*.toml", recursive=True):
    with open(file_path, "r") as f:
        content = f.read()
    
    content = re.sub(r'leak_rate\s*=\s*\d+', 'leak_shift = 4\nahp_amplitude = 0\nadaptive_leak_min_shift = 0', content)
    content = re.sub(r'adaptive_leak_max\s*=\s*\d+', '', content)
    content = re.sub(r'inertia_curve\s*=\s*\[[^\]]+\]', 'inertia_curve = [ 128, 108, 88, 66, 41, 22, 8, 0,]', content)
    
    # In case there are multiple empty lines created
    content = re.sub(r'\n{3,}', '\n\n', content)
    
    with open(file_path, "w") as f:
        f.write(content)

print("GNM-Library models updated successfully.")
