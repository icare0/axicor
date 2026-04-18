import numpy as np

class PwmDecoder:
    """
    Temporal PWM Decoding (Rate Coding) for motor cortex.
    Converts the binary spike history (Output_History) of a batch
    into a dense f16 array of analog efforts (Duty Cycle / 0.0 - 1.0).
    """
    def __init__(self, num_outputs: int, batch_size: int):
        self.N = num_outputs
        self.B = batch_size
        
        # Payload size: B ticks * N motors (1 byte = 1 spike flag)
        self.payload_size = self.N * self.B
        self._inv_b = np.float16(1.0 / self.B)
        
        # Preallocation for HFT cycle (Zero-Garbage)
        self._sum_buffer = np.zeros(self.N, dtype=np.float16)
        self._out_buffer = np.zeros(self.N, dtype=np.float16)

    def decode_from(self, rx_view: memoryview) -> np.ndarray:
        """
        Extracts data from raw UDP buffer without memory copying.
        rx_view: socket memoryview (header ALREADY stripped in client.step)
        """
        # Amnesia Defense: If no data, return zero effort
        if len(rx_view) == 0:
            self._out_buffer.fill(0.0)
            return self._out_buffer

        # 1. Zero-copy byte cast. Internally creates only a view on OS memory.
        raw_bytes = np.frombuffer(rx_view, dtype=np.uint8, count=self.payload_size, offset=0)
        
        # 2. Virtual reshape (Motors, Ticks). [Pixel][Batch]
        spikes_2d = raw_bytes.reshape((self.N, self.B))
        
        # 3. Vectorized sum across ticks axis (axis=1). Written directly into preallocated buffer!
        np.sum(spikes_2d, axis=1, dtype=np.float16, out=self._sum_buffer)
        
        # 4. Normalize to [0.0, 1.0] range (In-place)
        np.multiply(self._sum_buffer, self._inv_b, out=self._out_buffer)
        
        # Return reference to internal buffer. Data valid until next decode_from call.
        return self._out_buffer

class PopulationDecoder:
    """
    Population Decoder (Center of Mass) for extracting continuous float values
    from neuron receptive field activity.
    """
    def __init__(self, variables_count: int, neurons_per_var: int, batch_size: int):
        self.V = variables_count
        self.M = neurons_per_var
        self.N = self.V * self.M
        self.B = batch_size
        self.payload_size = self.N * self.B
        
        # Vector of receptive field centers [0.0 ... 1.0]
        self.centers = np.linspace(0.0, 1.0, self.M, dtype=np.float16)
        
        # Zero-Allocation Buffers
        self._sum_buffer = np.zeros((self.V, self.M), dtype=np.float16)
        self._mass_buffer = np.zeros(self.V, dtype=np.float16)
        self._out_buffer = np.zeros(self.V, dtype=np.float16)

    def decode_from(self, rx_view: memoryview) -> np.ndarray:
        # Amnesia Defense: Return neutral state (0.5)
        if len(rx_view) == 0:
            self._out_buffer.fill(0.5)
            return self._out_buffer

        # 1. Zero-copy byte cast
        raw_bytes = np.frombuffer(rx_view, dtype=np.uint8, count=self.payload_size, offset=0)
        
        # 2. Reshape (Variables, Neurons_per_Var, Batch)
        spikes_3d = raw_bytes.reshape((self.V, self.M, self.B))
        
        # 3. Sum spikes across ticks (Time Integration, axis=2)
        np.sum(spikes_3d, axis=2, dtype=np.float16, out=self._sum_buffer)
        
        # 4. Find total spike mass for each variable
        np.sum(self._sum_buffer, axis=1, out=self._mass_buffer)
        
        # 5. Weight activity by field centers (Broadcasting: (V, M) * (M,))
        np.multiply(self._sum_buffer, self.centers, out=self._sum_buffer)
        
        # 6. Sum weighted values
        np.sum(self._sum_buffer, axis=1, out=self._out_buffer)
        
        # 7. Center of Mass: Sum(spikes * centers) / Sum(spikes)
        np.divide(self._out_buffer, self._mass_buffer, out=self._out_buffer, where=self._mass_buffer != 0)
        
        # 8. Silence protection for specific variables (default to 0.5 if no spikes)
        mask = (self._mass_buffer == 0)
        self._out_buffer[mask] = 0.5
        
        return self._out_buffer
