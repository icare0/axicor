import os

files = [
    "GNM-Library/Cortex/L1/aspiny/MTG/3.toml",
    "GNM-Library/Cortex/L2/spiny/MFG/1.toml",
    "GNM-Library/Cortex/L3/aspiny/MFG/1.toml",
    "GNM-Library/Cortex/L5/spiny/MFG/1.toml",
    "GNM-Library/Cortex/L5/spiny/MFG/3.toml",
    "GNM-Library/Cortex/L5/aspiny/MFG/1.toml",
    "GNM-Library/Cortex/L6/spiny/MFG/1.toml",
    "GNM-Library/Cortex/L6/aspiny/MTG/2.toml",
]

out_path = "examples/cartpole/config/zones/MotorCortex/blueprints.toml"

formatted = ""
for path in files:
    with open(path, "r") as f:
        content = f.read()
    
    # Replace [[neuron_type]] with [[neuron_types]] and strip excess whitespace
    content = content.replace("[[neuron_type]]", "[[neuron_types]]").strip()
    
    formatted += content + "\n\n"

# Ensure output directory exists just in case
os.makedirs(os.path.dirname(out_path), exist_ok=True)

with open(out_path, "w") as f:
    f.write(formatted.strip() + "\n")

print(f"Successfully merged {len(files)} files into MotorCortex blueprints.")
