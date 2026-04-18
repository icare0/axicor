# Axicor Python SDK

The high-performance interface for connecting environments to the Axicor Neuromorphic Engine.

> ### CRITICAL: The 10ms Budget & Zero-Garbage Law
> Axicor operates on a hard real-time BSP (Barrier Synchronization) cycle. For most configurations, you have exactly 10ms to complete your observation-action loop. 
> 
> You MUST NOT perform any heap allocations (creating new numpy arrays, tuples, or large objects) inside the hot loop. Triggering the Python Garbage Collector (GC) will cause a micro-stutter, resulting in dropped UDP packets and "Biological Amnesia" where the brain loses its short-term state.

## Installation

```bash
pip install -e .
```

## Zero-Garbage Usage Example

The correct way to use the SDK involves pre-allocating all buffers outside the main loop and using the `_into` methods to perform zero-copy operations.

### The Correct Way (Zero-Allocation)

```python
import numpy as np
from axicor.encoders import PopulationEncoder
from axicor.client import AxicorMultiClient

# 1. Pre-allocate EVERYTHING outside the loop
encoder = PopulationEncoder(variables_count=4, neurons_per_var=16, batch_size=20)
client = AxicorMultiClient(addr=("127.0.0.1", 8081), ...)

# Pre-allocated observation buffer
obs_raw = np.zeros(4, dtype=np.float16)

while True:
    # Use pre-allocated observation
    obs_raw[:] = env.get_observation()
    
    # Encode DIRECTLY into the client's internal TX arena
    # No temporary arrays are created.
    encoder.encode_into(obs_raw, client.payload_views[0])
    
    # Step the simulation
    # Returns a memoryview of the pre-allocated RX arena
    action_view = client.step(reward=1.0)
    
    # Process actions (use memoryview directly or wrap in pre-allocated ndarray)
    apply_actions(action_view)
```

### The Wrong Way (Triggers GC)

```python
while True:
    obs = env.get_observation()
    
    # WRONG: Creating a new array inside the loop
    # This triggers malloc() and eventually the Garbage Collector.
    encoded_spikes = encoder.encode(obs) 
    
    # WRONG: This will cause UDP timeouts in the Node
    client.send(encoded_spikes)
```

## Performance Tracking

Use the provided `test_zero_gc.py` to verify that your implementation does not leak memory in the hot path:

```bash
python test_zero_gc.py
```

## License
GPL-3.0-or-later
