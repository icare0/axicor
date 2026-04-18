#  Axicor Client SDK: HFT Python Guide

   Client SDK.     , backpropagation  `loss.backward()`.      ,     .

    -  HFT- (Agent),              ,    **Lockstep-** .

---

## 1. Sensory-Motor Loop (HFT-)

          UDP Fast-Path.    JSON  .    .

> [!IMPORTANT]
> **  (Zero-Garbage)**
>    `while True:` **  **.   `[]`,   `float()`,   `for`  .      (GC) ,       10 .

###     Zero-Cost Facade

```python
import time
import numpy as np
from axicor.client import AxicorMultiClient
from axicor.contract import AxicorIoContract
from axicor.encoders import PopulationEncoder
from axicor.decoders import PwmDecoder

# 1.    
contract_an = AxicorIoContract("baked/SensoryCortex", "SensoryCortex")
cfg_in = contract_an.get_client_config(BATCH_SIZE=10)

client = AxicorMultiClient(addr=("127.0.0.1", 8081), matrices=cfg_in["matrices"], rx_layout=[])

#    (Zero-Garbage)
obs_padded = np.zeros(64, dtype=np.float16)
bounds = np.zeros((64, 2), dtype=np.float16)

# 2.  Zero-Cost    
avatar_in = contract_an.create_input_facade("sensors", obs_padded)

#     (   )
bounds[0] = [-50.0, 50.0]  # pos_x
bounds[1] = [-50.0, 50.0]  # pos_y

range_diff = bounds[:, 1] - bounds[:, 0]
range_diff[range_diff == 0] = 1.0

encoder = contract_an.create_population_encoder("sensors", vars_count=64, batch_size=10)

while True:
    # --- ZERO-GARBAGE HOT LOOP ---

    # 1.    (O(1)  ,  )
    avatar_in.pos_x = state["x"]
    avatar_in.pos_y = state["y"]

    # 2.    
    norm_state = np.clip((obs_padded - bounds[:, 0]) / range_diff, 0.0, 1.0)

    # 3.   VRAM (1  C-ABI)
    encoder.encode_into(norm_state, client.payload_views)
    rx = client.step(dopamine_signal)
```

###  Memory-Mapped Facade
 `avatar_in.pos_x = 5.0`   ,      C.   `create_input_facade`  `io.toml`   Python `@property` /,        `obs_padded`.

###       (DOD)

*   `PopulationEncoder`  `PwmDecoder`    `np.packbits`  `np.frombuffer`.      `memoryview`   .
*    `client.step()`    20-  `ExternalIoHeader`,   `recvfrom`,       `memoryview` .
*     Python- -  0.1 .        (Gymnasium).

---

## 2. The Neurosurgeon ( )

        (  ), SDK   `AxicorMemory`  `AxicorSurgeon`.       Zero-Copy mmap   (`/dev/shm`),    Rust.

> [!WARNING]
> :    memory     .        HFT.           .

###     (Grafting)
   (     `master_seed`),        .          :

```python
from axicor.memory import AxicorMemory
from axicor.surgeon import AxicorSurgeon

# 1.   VRAM   (read_only=False)
mem_donor = AxicorMemory(zone_hash=0xDEADBEEF, read_only=False)
surgeon = AxicorSurgeon(mem_donor)

# :   1D-   
offsets, signs = surgeon.extract_graft(threshold=25000)

# 2.   "" 
mem_recipient = AxicorMemory(zone_hash=0xBEEFDEAD, read_only=False)
surgeon_rec = AxicorSurgeon(mem_recipient)

#    ()
surgeon_rec.inject_graft(offsets, signs)
```

** :**      -. `extract_graft`   `np.where()`  mmap   ,    . `inject_graft`     (32767)   .      RAM.
