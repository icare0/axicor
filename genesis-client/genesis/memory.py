import os
import mmap
import struct
import numpy as np

from .platform import get_shm_path

class GenesisMemory:
    # Строгий C-ABI v3 (128 bytes) -> genesis-core/src/ipc.rs
    SHM_HEADER_FMT = "<IBBHIIIIQIIIIIIIIIII13I"
    SHM_HEADER_SIZE = 128
    MAGIC = 0x47454E53 # "GENS"

    def __init__(self, zone_hash: int, read_only: bool = False):
        self.zone_hash = zone_hash
        path = get_shm_path(zone_hash)
        
        mode = os.O_RDONLY if read_only else os.O_RDWR
        access = mmap.ACCESS_READ if read_only else mmap.ACCESS_WRITE
        
        fd = os.open(path, mode)
        self._mm = mmap.mmap(fd, 0, access=access)
        os.close(fd)
        
        # 1. Читаем заголовок
        header = struct.unpack_from(self.SHM_HEADER_FMT, self._mm, 0)
        assert header[0] == self.MAGIC, f"Invalid SHM Magic in {path}"
        
        self.padded_n = header[4]
        self.dendrite_slots = header[5]
        self.weights_offset = header[6]
        self.targets_offset = header[7]
        self.flags_offset = header[16]
        self.voltage_offset = header[17]
        self.threshold_offset_offset = header[18]
        self.timers_offset = header[19]
        
        assert self.dendrite_slots == 128, "C-ABI violation: dendrite_slots != 128"
        
        # 2. Натягиваем матрицы прямо на оперативную память ОС
        self.weights = np.ndarray(
            (self.dendrite_slots, self.padded_n), 
            dtype=np.int32, 
            buffer=self._mm, 
            offset=self.weights_offset
        )
        
        self.targets = np.ndarray(
            (self.dendrite_slots, self.padded_n), 
            dtype=np.uint32, 
            buffer=self._mm, 
            offset=self.targets_offset
        )
        
        self.flags = np.ndarray(
            (self.padded_n,),
            dtype=np.uint8,
            buffer=self._mm,
            offset=self.flags_offset
        )

        self.voltage = np.ndarray(
            (self.padded_n,),
            dtype=np.int32,
            buffer=self._mm,
            offset=self.voltage_offset
        )

        self.threshold_offset = np.ndarray(
            (self.padded_n,),
            dtype=np.int32,
            buffer=self._mm,
            offset=self.threshold_offset_offset
        )

        self.timers = np.ndarray(
            (self.padded_n,),
            dtype=np.uint8,
            buffer=self._mm,
            offset=self.timers_offset
        )

    def save_checkpoint(self, filepath: str):
        """
        Zero-Copy дамп физического состояния графа в сжатый архив NumPy.
        Сбрасывает веса, цели (топологию) и флаги гомеостаза.
        """
        np.savez_compressed(filepath, 
                            weights=self.weights, 
                            targets=self.targets, 
                            flags=self.flags)

    def load_checkpoint(self, filepath: str):
        """
        In-Place загрузка состояния из чекпоинта прямо в Shared Memory.
        Строго соблюдает C-ABI смещения, не разрывая связь с mmap.
        """
        # data — словарь объектов из архива
        with np.load(filepath) as data:
            # Shape Safety: Защита от Silent Data Corruption при несовпадении коннектомов
            assert data['weights'].shape == self.weights.shape, f"Weights shape mismatch: {data['weights'].shape} vs {self.weights.shape}"
            assert data['targets'].shape == self.targets.shape, f"Targets shape mismatch: {data['targets'].shape} vs {self.targets.shape}"
            assert data['flags'].shape == self.flags.shape, f"Flags shape mismatch: {data['flags'].shape} vs {self.flags.shape}"

            # ЗАКОН: Strict In-Place Copy. 
            # Нельзя делать self.weights = data['weights'], это убьет mmap!
            self.weights[:] = data['weights']
            self.targets[:] = data['targets']
            self.flags[:] = data['flags']

            # Форсируем сброс страниц памяти в /dev/shm для мгновенной реакции ядра Rust
            self._mm.flush()

    @staticmethod
    def pack_targets(axon_ids: np.ndarray, segment_offsets: np.ndarray) -> np.ndarray:
        """
        Строгий C-ABI Packer (Zero-Index Trap Protection).
        [31..24] segment_offset (8 bit) | [23..0] axon_id + 1 (24 bit)
        """
        assert np.all(axon_ids >= 0) and np.all(axon_ids <= 0x00FFFFFE), "axon_id out of range (0..16777214)"
        assert np.all(segment_offsets >= 0) and np.all(segment_offsets <= 255), "segment_offset out of range (0..255)"
        
        return (segment_offsets.astype(np.uint32) << 24) | (axon_ids.astype(np.uint32) + 1)

    @staticmethod
    def unpack_targets(packed_targets: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
        """
        Строгий C-ABI Unpacker.
        Возвращает: (axon_ids, segment_offsets)
        """
        axon_ids = (packed_targets & 0x00FFFFFF).astype(np.int32) - 1
        segment_offsets = packed_targets >> 24
        return axon_ids, segment_offsets

    def extract_topology(self, soma_idx: int) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
        """
        Векторизованное извлечение геометрии для конкретного нейрона.
        Решает проблему Zero-Index Trap без циклов Python.
        Возвращает: (axon_ids, segment_offsets, weights)
        """
        # Срез колонки дендритов для одной сомы (O(1) pointer view)
        raw_targets = self.targets[:, soma_idx]
        raw_weights = self.weights[:, soma_idx]
        
        # Фильтруем пустые слоты
        valid_mask = raw_targets != 0
        valid_targets = raw_targets[valid_mask]
        active_weights = raw_weights[valid_mask]
        
        # Распаковываем u32 через статический метод
        axon_ids, seg_offsets = self.unpack_targets(valid_targets)
        
        return axon_ids, seg_offsets, active_weights

    def get_network_stats(self) -> dict:
        """Сканирует VRAM (Zero-Copy) и возвращает статистику графа."""
        # Убираем нулевые цели (пустые слоты)
        valid_mask = self.targets != 0
        active_weights = self.weights[valid_mask]
        
        if len(active_weights) == 0:
            return {"active_synapses": 0, "avg_weight": 0.0, "max_weight": 0}
            
        # [DOD FIX] Shift Mass Domain back to Charge Domain for human-readable telemetry
        return {
            "active_synapses": int(np.sum(valid_mask)),
            "avg_weight": float(np.mean(np.abs(active_weights))) / 65536.0,
            "max_weight": int(np.max(np.abs(active_weights))) // 65536
        }

    def distill_graph(self, prune_threshold: int) -> int:
        # [DOD FIX] Перевод порога в Mass Domain (i32)
        mass_threshold = prune_threshold << 16
        
        # Векторизованный поиск слабого мусора
        weak_mask = (self.targets != 0) & (np.abs(self.weights) < mass_threshold)
        pruned_count = int(np.sum(weak_mask))
        
        # Физическое уничтожение
        self.targets[weak_mask] = 0
        self.weights[weak_mask] = 0
        
        return pruned_count

    def clear_weights(self):
        """
        Tabula Rasa: Полное обнуление всех весов в шарде.
        Оставляет топологию (связи), но делает их "невесомыми".
        """
        self.weights.fill(0)

    def close(self):
        self._mm.close()
