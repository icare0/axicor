import os

# Files to merge
files = [
    "GNM-Library/Cortex/L1/aspiny/VISp1/1.toml",
    "GNM-Library/Cortex/L23/spiny/VISp23/16.toml",
    "GNM-Library/Cortex/L23/aspiny/VISp23/1.toml",
    "GNM-Library/Cortex/L4/spiny/VISp4/1.toml",
    "GNM-Library/Cortex/L4/aspiny/VISp4/18.toml",
    "GNM-Library/Cortex/L5/spiny/VISp5/12.toml",
    "GNM-Library/Cortex/L5/aspiny/VISp5/1.toml",
    "GNM-Library/Cortex/L6a/spiny/VISp6a/21.toml",
    "GNM-Library/Cortex/L6a/aspiny/VISl6a/1.toml",
]

out_path = "examples/cartpole/config/zones/SensoryCortex/blueprints.toml"

with open(out_path, "w") as out:
    for path in files:
        with open(path, "r") as f:
            content = f.read()
        out.write(content)
        if not content.endswith("\n"):
            out.write("\n")

print(f"Successfully concatenated {len(files)} files into {out_path}.")
