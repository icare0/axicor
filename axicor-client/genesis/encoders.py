import math
import numpy as np

class PwmEncoder:
    """
    Temporal PWM Encoding (Rate Coding) for continuous analog signals.
    Spreads spikes across the batch via phase shifting, preventing Burst Gating.
    """
    def __init__(self, num_sensors: int, batch_size: int):
        self.N = num_sensors
        self.B = batch_size
        
        # GPU expects an array of u32 (32 virtual axons per word).
        # Each tick row must be a multiple of 4 bytes (32 bits).
        self.padded_N = math.ceil(self.N / 64) * 64
        self.bytes_per_tick = self.padded_N // 8
        self.total_bytes = self.bytes_per_tick * self.B
        
        # Temporal axis and phase shift (Golden Ratio Dither)
        t = np.linspace(0, 1, self.B, endpoint=False, dtype=np.float16)[:, None]
        phase = (np.arange(self.N, dtype=np.float16) * 0.618033) % 1.0
        self.pwm_wave = (t + phase) % 1.0
        
        # Preallocated buffer to avoid heap allocations in the Hot Loop
        self._bool_buffer = np.zeros((self.B, self.padded_N), dtype=np.bool_)

    def encode_into(self, sensors_f16: np.ndarray, tx_view: memoryview) -> int:
        """
        Broadcasting comparison and Zero-Copy write to the network buffer.
        Returns the number of bytes written.
        """
        # [DOD FIX] Strict Zero-Allocation comparison.
        # No temporary arrays! The result is written directly into _bool_buffer.
        np.less(self.pwm_wave, sensors_f16, out=self._bool_buffer[:, :self.N])

        # bitorder='little' is critical! CUDA does (word >> bit_idx) & 1
        packed = np.packbits(self._bool_buffer, bitorder='little', axis=1)

        # Copy flat byte array directly into the UDP socket buffer
        tx_view[:self.total_bytes] = packed.ravel()
        return self.total_bytes

class PopulationEncoder:
    """
    Spatial encoding (Gaussian Receptive Fields).
    Expands 1 float variable into a population of M neurons.
    """
    def __init__(self, variables_count: int, neurons_per_var: int, batch_size: int, sigma: float = 0.15):
        self.V = variables_count
        self.M = neurons_per_var
        self.N = self.V * self.M
        self.B = batch_size
        
        self.padded_N = math.ceil(self.N / 64) * 64
        self.bytes_per_tick = self.padded_N // 8
        self.total_bytes = self.bytes_per_tick * self.B
        
        # Precalculate activation radius for Gaussian. prob > 0.5 is equivalent to abs(dist) < R
        self.sigma = sigma
        self.radius = self.sigma * math.sqrt(-2.0 * math.log(0.5))
        
        # [DOD FIX] Preallocation of buffers for in-place calculations (Zero-Garbage)
        self.centers = np.linspace(0.0, 1.0, self.M, dtype=np.float16)
        self._expanded_buffer = np.zeros(self.N, dtype=np.float16)
        self._bool_buffer = np.zeros(self.padded_N, dtype=np.bool_)
        self._batch_bool_buffer = np.zeros((self.B, self.padded_N), dtype=np.bool_)
    def encode_into(self, states_f16: np.ndarray, tx_view: memoryview) -> int:
        """
        states_f16: array of normalized [0..1] values (size V)
        """
        # [DOD FIX] Zero-Allocation math pipeline
        view_2d = self._expanded_buffer.reshape(self.V, self.M)
        view_2d[:] = states_f16[:, None]

        # Vectorized subtraction of centers In-Place
        np.subtract(view_2d, self.centers, out=view_2d)
        np.abs(self._expanded_buffer, out=self._expanded_buffer)

        # Threshold activation In-Place
        np.less(self._expanded_buffer, self.radius, out=self._bool_buffer[:self.N])

        # [DOD Task 1] Single-Tick Pulse: write only to the zero tick of the batch
        # Other ticks remain filled with False (biological silence)
        self._batch_bool_buffer[0, :] = self._bool_buffer

        packed = np.packbits(self._batch_bool_buffer, bitorder='little', axis=1)

        tx_view[:self.total_bytes] = packed.ravel()
        return self.total_bytes
        
