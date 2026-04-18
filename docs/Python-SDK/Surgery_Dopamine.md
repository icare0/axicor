# Dopamine Injection & The AxicorSurgeon (Python SDK)

 Axicor       (backpropagation),   **    (R-STDP)**        (VRAM).

## 1.   (Time-Scaled R-STDP)

  () -   ,      UDP- Data Plane.     `model.train()`  `model.eval()`.   .

    (     `32767`),    **Time-Scaled R-STDP**:

1. **Background Erosion ( ):**        (LTD).
2. **Phasic Reward ( ):**      (LTP).
3. **Pain Shock ( ):**      .       .

###    Hot Loop

```python
#  
DOPAMINE_PULSE = -25         #   (  )
DOPAMINE_REWARD = 30         #     
DOPAMINE_PUNISHMENT = -255   # Death Signal ( )

while True:
    # ...  state    ...

    if terminated or truncated:
        # THE DEATH SIGNAL:  
        #    15 ,  C++  GSOP  
        #    
        for _ in range(15):
            client.step(DOPAMINE_PUNISHMENT)
        
        env.reset()
        continue

    #   (Phasic)
    if score > 0 and score % 10 == 0:
        dopamine_signal = DOPAMINE_REWARD
    else:
        dopamine_signal = DOPAMINE_PULSE

    #    -  Lockstep
    rx_view = client.step(dopamine_signal)
```

###      (C-ABI)
SDK        `client.step()`,      (i16)    20-  `ExternalIoHeader`    16   `struct.pack_into("<IIIIhH", ...)`.  JSON -  C-   .

---

## 2. AxicorSurgeon:   
 `AxicorSurgeon` -  Data-Oriented .       Zero-Copy mmap  `/dev/shm/axicor_shard_*`,     Rust-.

> [!WARNING]
>  : `AxicorSurgeon`      (Hot Loop) .        FPS   10  Lockstep.        -.

### 2.1. GABA Incubation (  )
   (Tabula Rasa)    ,      - (Sensory Flooding).      (Inhibitory) :

```python
from axicor.memory import AxicorMemory
from axicor.surgeon import AxicorSurgeon

#   VRAM- 
mem = AxicorMemory(zone_hash=0xDEADBEEF, read_only=False)
surgeon = AxicorSurgeon(mem)

#     (  ==  )
#      .
#    NumPy  .
surgeon.incubate_gaba(baseline_weight=-30000)
```

### 2.2.   (Path-Based Extraction)

        .     (,  )  **Vectorized Back-Tracing**.

    ()       .  C-ABI  VRAM     (Forward Pass)      `axon_to_soma`,      O(1)   Python     mmap.   `for`    NumPy,    L1/L2  -.

**   GABA-:**
       (`abs(weight) > threshold`).   Axicor        ( ),   `abs()`      ** (Inhibitory)** .  ,                .

```python
# 1.  " " ()  
#  ID  ,     (  .gxo)
motor_soma_ids = np.array([1, 2])
payload = source_surgeon.extract_reflex_path(motor_soma_ids, prune_threshold=15000)
```

### 2.3.     (Surgical Grafting)

     :           - (  ),   .

**:**   ()          R-STDP -  ,    .     15-   (`abs(w) = 32767`).    GSOP,    ""       .

```python
# 2.   
target_surgeon.inject_subgraph(payload)
```

> [!CAUTION]
> **  (Zero-Index Trap):**    `dendrite_targets`   ,  `target == 0`     Early Exit  GPU.  `axon_id`    `+1` (`axon_id = (target & 0x00FFFFFF) - 1`).    `axon_id`             `0xFFFFFFFF`  Segmentation Fault.
