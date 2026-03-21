import numpy as np

class PwmDecoder:
    """
    Temporal PWM Decoding (Rate Coding) для моторного кортекса.
    Конвертирует бинарную историю спайков (Output_History) за батч
    в плотный f16 массив аналоговых усилий (Duty Cycle / 0.0 - 1.0).
    """
    def __init__(self, num_outputs: int, batch_size: int):
        self.N = num_outputs
        self.B = batch_size
        
        # Размер полезной нагрузки: B тиков * N моторов (1 байт = 1 флаг спайка)
        self.payload_size = self.N * self.B
        self._inv_b = np.float16(1.0 / self.B)
        
        # Преаллокация для HFT-цикла (Zero-Garbage)
        self._sum_buffer = np.zeros(self.N, dtype=np.float16)
        self._out_buffer = np.zeros(self.N, dtype=np.float16)

    def decode_from(self, rx_view: memoryview, offset: int) -> np.ndarray:
        """
        Извлекает данные из сырого UDP буфера без копирования памяти.
        rx_view: memoryview сокета
        offset: размер заголовка C-ABI (обычно 20 байт)
        """
        # Amnesia Defense: Если данных нет, возвращаем нулевое усилие
        if len(rx_view) == 0:
            self._out_buffer.fill(0.0)
            return self._out_buffer

        # 1. Zero-copy каст байтов. Под капотом создается только view на память ОС.
        raw_bytes = np.frombuffer(rx_view, dtype=np.uint8, count=self.payload_size, offset=offset)
        
        # 2. Виртуальный reshape (Тики, Моторы). Меняет только strides, без копирования.
        spikes_2d = raw_bytes.reshape((self.B, self.N))
        
        # 3. Векторизованная сумма по оси тиков (по времени). Запись прямо в преаллоцированный буфер!
        np.sum(spikes_2d, axis=0, dtype=np.float16, out=self._sum_buffer)
        
        # 4. Нормализация к диапазону [0.0, 1.0] (In-place)
        np.multiply(self._sum_buffer, self._inv_b, out=self._out_buffer)
        
        # Возвращаем ссылку на внутренний буфер. Данные валидны до следующего вызова decode_from.
        return self._out_buffer

class PopulationDecoder:
    """
    Population Decoder (Center of Mass) для извлечения непрерывных float-значений
    из активности рецептивных полей нейронов.
    """
    def __init__(self, variables_count: int, neurons_per_var: int, batch_size: int):
        self.V = variables_count
        self.M = neurons_per_var
        self.N = self.V * self.M
        self.B = batch_size
        self.payload_size = self.N * self.B
        
        # Вектор центров рецептивных полей [0.0 ... 1.0]
        self.centers = np.linspace(0.0, 1.0, self.M, dtype=np.float16)
        
        # Zero-Allocation Буферы
        self._sum_buffer = np.zeros((self.V, self.M), dtype=np.float16)
        self._mass_buffer = np.zeros(self.V, dtype=np.float16)
        self._out_buffer = np.zeros(self.V, dtype=np.float16)

    def decode_from(self, rx_view: memoryview, offset: int) -> np.ndarray:
        # Amnesia Defense: Возвращаем нейтральное состояние (0.5)
        if len(rx_view) == 0:
            self._out_buffer.fill(0.5)
            return self._out_buffer

        # 1. Zero-copy каст байтов
        raw_bytes = np.frombuffer(rx_view, dtype=np.uint8, count=self.payload_size, offset=offset)
        
        # 2. Reshape (Batch, Variables, Neurons_per_Var)
        spikes_3d = raw_bytes.reshape((self.B, self.V, self.M))
        
        # 3. Суммируем спайки по тикам (Time Integration)
        np.sum(spikes_3d, axis=0, dtype=np.float16, out=self._sum_buffer)
        
        # 4. Находим общую массу спайков на каждую переменную
        np.sum(self._sum_buffer, axis=1, out=self._mass_buffer)
        
        # 5. Взвешиваем активность центрами полей (Broadcasting: (V, M) * (M,))
        np.multiply(self._sum_buffer, self.centers, out=self._sum_buffer)
        
        # 6. Складываем взвешенные значения
        np.sum(self._sum_buffer, axis=1, out=self._out_buffer)
        
        # 7. Центр масс: Sum(spikes * centers) / Sum(spikes)
        np.divide(self._out_buffer, self._mass_buffer, out=self._out_buffer, where=self._mass_buffer != 0)
        
        # 8. Защита от тишины в конкретной переменной (если нет спайков, ставим 0.5)
        mask = (self._mass_buffer == 0)
        self._out_buffer[mask] = 0.5
        
        return self._out_buffer
