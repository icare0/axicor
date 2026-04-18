# High-Performance Encoders & Decoders (Python SDK)

 Axicor           100+ .     `for`   - `Spike` -      (GIL       10 ).

 HFT- - **Data-Oriented Design (DOD)**.   float-         NumPy     `memoryview`     Hot Loop.

SDK   Zero-Copy .

## 1.   (Encoders)

  (`float16`)      Python-      (`8081`).

### 1.1. PopulationEncoder ( )
  `float`     `M`  (Gaussian Receptive Fields).   ,    (,  CartPole).

```python
from axicor.encoders import PopulationEncoder

# 4    16   ,  = 10 
encoder = PopulationEncoder(variables_count=4, neurons_per_var=16, batch_size=10)

#  Hot Loop:
# 1.    [0.0, 1.0] (In-place)
norm_state = np.clip(state / max_bounds, 0.0, 1.0)

# 2.      !
# offset=20    C-ABI  ExternalIoHeader
encoder.encode_into(norm_state, client.payload_views, offset=20)
```

**DOD-:**  `encode_into`  `broadcast_to`         `self._expanded_buffer`.       .

### 1.2. PwmEncoder (  / Rate Coding)
    (RGB-, ).  f16 (0.0 - 1.0)       .

> [!IMPORTANT]
> **  Burst Gating:** `PwmEncoder`    (Golden Ratio Dither),     .      `synapse_refractory_period`.

```python
from axicor.encoders import PwmEncoder

# 1024 ,   = 100 
pwm = PwmEncoder(num_sensors=1024, batch_size=100)

#  Hot Loop:
pwm.encode_into(camera_frame_f16, client.payload_views, offset=20)
```

### 1.3. RetinaEncoder (Event-Driven Vision Pipeline)

     .       ()       (Center-Surround Antagonism). `RetinaEncoder`    - ,   RGB/Depth     .

**   (DOD):**
1. **Zero-Garbage OpenCV:**  `cv2` ,     `dst=`.    (`_gray`, `_center`, `_surround`, `_dog`)   .          GC   10-  HFT.
2. **Single-Tick Pulse ( ):**        .      `tick 0`  .     .    VRAM (Burst Gating)       .
3. **C-ABI Warp Alignment:**    ,        32  (`math.ceil(N / 32) * 32`),         GPU.

```python
from axicor.retina import RetinaEncoder
import cv2

#  ( Hot Loop).  .
# 256x256 ,  20 .
retina = RetinaEncoder(width=256, height=256, batch_size=20, threshold=15.0)

while True:
    # 1.   BGR- (,    )
    frame_bgr = env.render() 
    
    # 2. In-Place : BGR -> Grayscale -> DoG (Difference of Gaussians) -> 
    #  Little-Endian       
    retina.encode_into(frame_bgr, client.payload_views, offset=20)
```

**   (DoG):**       .   ( )   .   in-place     (Surround)    (Center)  `np.subtract(..., out=...)`    `np.greater(..., out=...)`.

### 1.4.    (C-ABI & Routing)

  Zero-Copy      VRAM , `RetinaEncoder`      Axicor.
**1. Feature Pyramid Batching ( 05 2.3):**
     .     (Features),      .      Burst Gating  VRAM.
*   **Tick 0**:   (Difference of Gaussians).
*   **Tick 1**:   (Frame Delta).
*   **Tick 2**:  R-G (Opponent Red-Green).
*   **Tick 3**:  B-Y (Opponent Blue-Yellow).
*   **Tick 4..N**:   ().

**2. Adaptive Global Inhibition ( ):**
          .
*   ****: `RetinaEncoder`     (  `cv2.mean`)     `threshold`   . 
*   ****:  `ExternalIoHeader`   20 .      .        FPS.

**3. Active Vision (Roaming Fovea):**
     (Window of Attention)   Python-     GPU.
*   **Fovea Matrix**:  `BrainBuilder`      (. 64x64).
*   **Gaze Control**:   `fovea_motors`   (X, Y)  Python-.
*   **Dynamic Slicing**: `RetinaEncoder`  In-Place     (. 8K)        .     ,    UDP  VRAM.

**4. Warp Alignment ( ):**
  100% Coalesced Access   `InjectInputs`,         32  (4 ).
*   ****: `bytes_per_tick = ceil((W * H) / 32) * 4`.
`RetinaEncoder`    .       Out-Of-Bounds   GPU.

**5. L7-Chunking  MTU:**
...
    (, 256x256 = 65 536  ). `BrainBuilder`      .
     L7-,      `uv_rect` (   0.0  1.0). UDP-  `axicor-baker`     . `RetinaEncoder`     memoryview    .

---

## 2.   (Decoders)
    ,   `Output_History` (   ).  `RecordReadout`  1  (u8)   .

### PwmDecoder (Rate Decoding)
       float-  (Duty Cycle)   .

```python
from axicor.decoders import PwmDecoder

# 128  ,  = 10 
decoder = PwmDecoder(num_outputs=128, batch_size=10)

#  Hot Loop ( sock.recvfrom_into   ):
#    ,   reshape(10, 128) 
#       (axis=0)  !
motor_forces = decoder.decode_from(rx_view, offset=20)

#  (Winner-Takes-All)
left_force = np.sum(motor_forces[:64])
right_force = np.sum(motor_forces[64:])
action = 0 if left_force > right_force else 1
```

###   HFT- (Pro Tips)

1. Zero-Copy Sockets:    sock.recv(65535).    bytes  .  sock.recvfrom_into(rx_buf)    bytearray    memoryview. ( AxicorMultiClient    ).
2. C-ABI Offset:  ,   20    Data Plane -  ExternalIoHeader (<IIIIhH).         20.
3. In-Place Math:    ,   out=   NumPy: np.multiply(raw, 0.5, out=preallocated_array).