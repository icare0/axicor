# axicor-baker

The offline topology compiler for Axicor, responsible for compiling high-level TOML brain descriptions into optimized binary C-ABI memory dumps.

## Technical Focus

- **Cone Tracing Algorithm:** A high-performance 3D routing algorithm used to grow axons through the connectome space and establish synaptic connections based on biological proximity.
- **TOML DNA Parsing:** Interprets human-readable brain blueprints, including layer definitions, neuron populations, and spatial projection rules.
- **Spatial Hashing:** Uses advanced spatial partitioning to accelerate the detection of potential synapses during the baking process.
- **Archive Generation:** Packages the resulting memory dumps (.state, .axons) into single .axic archives ready for deployment to nodes.

## Workflow

1. **Topology Generation:** Define zones and layers in TOML.
2. **Baking:** Run `axicor-baker` to perform axon routing and soma-to-axon mapping.
3. **Deployment:** Load the .axic archive into `axicor-node`.

## License
Dual-licensed under MIT or Apache 2.0.
