import os

files = [
    "examples/cartpole/config/zones/HiddenCortex/blueprints.toml",
    "examples/cartpole/config/zones/MotorCortex/blueprints.toml"
]

for path in files:
    with open(path, "r") as f:
        content = f.read()
    
    # Revert the typo
    content = content.replace("[[neuron_types]]", "[[neuron_type]]")
    
    with open(path, "w") as f:
        f.write(content)

print("Reverted to [[neuron_type]] in both files.")
