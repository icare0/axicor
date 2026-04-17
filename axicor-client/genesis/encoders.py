import math
import numpy as np

class PwmEncoder:
    """
    Temporal PWM Encoding (Rate Coding) для непрерывных аналоговых сигналов.
    Размазывает спайки по батчу через фазовый сдвиг, предотвращая Burst Gating.
    """
    def __init__(self, num_sensors: int, batch_size: int):
        self.N = num_sensors
        self.B = batch_size
        
        # GPU ожидает массив u32 (по 32 виртуальных аксона в слове).
        # Строка каждого тика обязана быть кратна 4 байтам (32 битам).
        self.padded_N = math.ceil(self.N / 64) * 64
        self.bytes_per_tick = self.padded_N // 8
        self.total_bytes = self.bytes_per_tick * self.B
        
        # Временная ось и фазовый сдвиг (Golden Ratio Dither)
        t = np.linspace(0, 1, self.B, endpoint=False, dtype=np.float16)[:, None]
        phase = (np.arange(self.N, dtype=np.float16) * 0.618033) % 1.0
        self.pwm_wave = (t + phase) % 1.0
        
        # Преаллоцированный буфер для избежания аллокаций в Hot Loop
        self._bool_buffer = np.zeros((self.B, self.padded_N), dtype=np.bool_)

    def encode_into(self, sensors_f16: np.ndarray, tx_view: memoryview) -> int:
        """
        Broadcasting сравнение и Zero-copy запись в сетевой буфер.
        Возвращает количество записанных байт.
        """
        # [DOD FIX] Strict Zero-Allocation comparison.
        # Никаких временных массивов! Результат пишется прямо в _bool_buffer.
        np.less(self.pwm_wave, sensors_f16, out=self._bool_buffer[:, :self.N])

        # bitorder='little' критичен! CUDA делает (word >> bit_idx) & 1
        packed = np.packbits(self._bool_buffer, bitorder='little', axis=1)

        # Копирование плоского массива байт прямо в UDP сокет-буфер
        tx_view[:self.total_bytes] = packed.ravel()
        return self.total_bytes

class PopulationEncoder:
    """
    Пространственное кодирование (Gaussian Receptive Fields).
    Разворачивает 1 float переменную в популяцию из M нейронов.
    """
    def __init__(self, variables_count: int, neurons_per_var: int, batch_size: int, sigma: float = 0.15):
        self.V = variables_count
        self.M = neurons_per_var
        self.N = self.V * self.M
        self.B = batch_size
        
        self.padded_N = math.ceil(self.N / 64) * 64
        self.bytes_per_tick = self.padded_N // 8
        self.total_bytes = self.bytes_per_tick * self.B
        
        # Предрасчет радиуса активации для Гауссианы. prob > 0.5 эквивалентно abs(dist) < R
        self.sigma = sigma
        self.radius = self.sigma * math.sqrt(-2.0 * math.log(0.5))
        
        # [DOD FIX] Преаллокация буферов для in-place вычислений (Zero-Garbage)
        self.centers = np.linspace(0.0, 1.0, self.M, dtype=np.float16)
        self._expanded_buffer = np.zeros(self.N, dtype=np.float16)
        self._bool_buffer = np.zeros(self.padded_N, dtype=np.bool_)
        self._batch_bool_buffer = np.zeros((self.B, self.padded_N), dtype=np.bool_)
    def encode_into(self, states_f16: np.ndarray, tx_view: memoryview) -> int:
        """
        states_f16: массив нормализованных [0..1] значений (размер V)
        """
        # [DOD FIX] Zero-Allocation math pipeline
        view_2d = self._expanded_buffer.reshape(self.V, self.M)
        view_2d[:] = states_f16[:, None]

        # Векторизованное вычитание центров In-Place
        np.subtract(view_2d, self.centers, out=view_2d)
        np.abs(self._expanded_buffer, out=self._expanded_buffer)

        # Пороговая активация In-Place
        np.less(self._expanded_buffer, self.radius, out=self._bool_buffer[:self.N])

        # [DOD Task 1] Single-Tick Pulse: пишем только в нулевой тик батча
        # Остальные тики остаются заполнены False (биологическая тишина)
        self._batch_bool_buffer[0, :] = self._bool_buffer

        packed = np.packbits(self._batch_bool_buffer, bitorder='little', axis=1)

        tx_view[:self.total_bytes] = packed.ravel()
        return self.total_bytes
        
