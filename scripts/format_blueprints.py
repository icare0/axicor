import sys

input_file = "examples/cartpole/config/zones/SensoryCortex/blueprints.toml"

with open(input_file, 'r') as f:
    content = f.read()

# Replace any occurrence of `[[neuron_type]]` that lacks spacing before it.
# Easiest way: split by `[[neuron_type]]` and rejoin with newlines.
parts = content.split('[[neuron_type]]')

# Strip leading/trailing whitespaces from parts to normalize
parts = [p.strip() for p in parts if p.strip()]

# Rejoin with two newlines before `[[neuron_type]]`
formatted = ""
for p in parts:
    formatted += "[[neuron_type]]\n" + p + "\n\n"

with open(input_file, 'w') as f:
    f.write(formatted)

print("Formatted blueprints successfully.")
