import os

files = [
    "GNM-Library/Cortex/L1/aspiny/MTG/3.toml",
    "GNM-Library/Cortex/L2/spiny/MTG/4.toml",
    "GNM-Library/Cortex/L2/aspiny/MTG/1.toml",
    "GNM-Library/Cortex/L3/spiny/MTG/1.toml",
    "GNM-Library/Cortex/L3/aspiny/MTG/10.toml",
    "GNM-Library/Cortex/L4/spiny/MTG/1.toml",
    "GNM-Library/Cortex/L4/aspiny/MTG/10.toml",
    "GNM-Library/Cortex/L5/spiny/MTG/1.toml",
    "GNM-Library/Cortex/L5/aspiny/MTG/10.toml",
]

out_path = "examples/cartpole/config/zones/HiddenCortex/blueprints.toml"

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

print(f"Successfully merged {len(files)} files into HiddenCortex blueprints.")
