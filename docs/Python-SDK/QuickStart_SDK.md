# 🐍 Axicor Client SDK: HFT Python Guide

Добро пожаловать в Client SDK. Забудьте про статические графы вычислений, backpropagation и `loss.backward()`. Вы подключаетесь к живому биологическому реактору, который работает в реальном времени.

Ваша задача как инженера - написать HFT-шлюз (Agent), который будет переводить данные среды в спайки и успевать делать это за доли миллисекунды, чтобы не сорвать **Lockstep-барьер** кластера.

---

## 1. Sensory-Motor Loop (HFT-Пайплайн)

Классический цикл обучения заменяется на непрерывный обмен сырыми байтами через UDP Fast-Path. Мы не передаем JSON или списки. Мы шлём битовые маски.

> [!IMPORTANT]
> **ГЛАВНЫЙ ЗАКОН (Zero-Garbage)**
> Внутри горячего цикла `while True:` **запрещено создавать объекты**. Никаких списков `[]`, никаких кастов `float()`, никаких циклов `for` по сенсорам. Любая аллокация разбудит сборщик мусора (GC) Питона, и вы убьете жесткий бюджет в 10 мс.

### Эталонный цикл с использованием Zero-Cost Facade

```python
import time
import numpy as np
from genesis.client import GenesisMultiClient
from genesis.contract import GenesisIoContract
from genesis.encoders import PopulationEncoder
from genesis.decoders import PwmDecoder

# 1. Загрузка контрактов и преаллокация
contract_an = GenesisIoContract("baked/SensoryCortex", "SensoryCortex")
cfg_in = contract_an.get_client_config(BATCH_SIZE=10)

client = GenesisMultiClient(addr=("127.0.0.1", 8081), matrices=cfg_in["matrices"], rx_layout=[])

# Плоский буфер аватара (Zero-Garbage)
obs_padded = np.zeros(64, dtype=np.float16)
bounds = np.zeros((64, 2), dtype=np.float16)

# 2. Натягивание Zero-Cost Фасадов на сырую память
avatar_in = contract_an.create_input_facade("sensors", obs_padded)

# Физические лимиты для нормализации (исключаем деление на ноль)
bounds[0] = [-50.0, 50.0]  # pos_x
bounds[1] = [-50.0, 50.0]  # pos_y

range_diff = bounds[:, 1] - bounds[:, 0]
range_diff[range_diff == 0] = 1.0

encoder = contract_an.create_population_encoder("sensors", vars_count=64, batch_size=10)

while True:
    # --- ZERO-GARBAGE HOT LOOP ---

    # 1. Запись через Фасад (O(1) сдвиг указателя, без словарей)
    avatar_in.pos_x = state["x"]
    avatar_in.pos_y = state["y"]

    # 2. Векторизованная нормализация всего массива
    norm_state = np.clip((obs_padded - bounds[:, 0]) / range_diff, 0.0, 1.0)

    # 3. Транспорт в VRAM (1 вызов C-ABI)
    encoder.encode_into(norm_state, client.payload_views)
    rx = client.step(dopamine_signal)
```

### Паттерн Memory-Mapped Facade
Синтаксис `avatar_in.pos_x = 5.0` выглядит как ООП, но работает со скоростью сырого C. Под капотом `create_input_facade` читает `io.toml` и генерирует Python `@property` геттеры/сеттеры, которые жестко привязаны к индексам преаллоцированного массива `obs_padded`.

### ⚙️ Как это работает под капотом (DOD)

*   `PopulationEncoder` и `PwmDecoder` используют векторизованные операции `np.packbits` и `np.frombuffer`. Они пишут биты напрямую в `memoryview` сырого байтового буфера.
*   Метод `client.step()` пакует дофамин в 20-байтовый заголовок `ExternalIoHeader`, блокируется на `recvfrom`, отрезает заголовок ответа и возвращает чистый `memoryview` выходов.
*   Время выполнения Python-обвязки - менее 0.1 мс. Оставшееся время тратится исключительно на физику среды (Gymnasium).

---

## 2. The Neurosurgeon (Нейрохирургия коннектома)

Если вам нужно грубо вмешаться в память сети (или извлечь аналитику), SDK предоставляет модуль `GenesisMemory` и `GenesisSurgeon`. Они общаются с памятью строго через Zero-Copy mmap файлов ОС (`/dev/shm`), минуя оркестратор на Rust.

> [!WARNING]
> ПРЕДОХРАНИТЕЛЬ: Запрещено вызывать методы memory внутри горячего цикла каждый шаг. Обработка матриц в сотни мегабайт нарушит бюджет HFT. Вызывайте их только в моменты падения агента или при холодном старте.

### Извлечение и Трансплантация Навыков (Grafting)
Благодаря Структурному Детерминизму (если зоны скомпилированы с одинаковым `master_seed`), нейроны лежат по одним и тем же адресам. Вы можете дистиллировать обученный навык из одного агента в другого:

```python
from genesis.memory import GenesisMemory
from genesis.surgeon import GenesisSurgeon

# 1. Подключаемся к VRAM обученного агента (read_only=False)
mem_donor = GenesisMemory(zone_hash=0xDEADBEEF, read_only=False)
surgeon = GenesisSurgeon(mem_donor)

# Экстракция: извлекаем плоские 1D-оффсеты самых сильных синапсов
offsets, signs = surgeon.extract_graft(threshold=25000)

# 2. Подключаемся к "чистому" агенту
mem_recipient = GenesisMemory(zone_hash=0xBEEFDEAD, read_only=False)
surgeon_rec = GenesisSurgeon(mem_recipient)

# Мгновенная пересадка навыка (Монументализация)
surgeon_rec.inject_graft(offsets, signs)
```

**Под капотом:** Мы не обходим графы и ООП-узлы. `extract_graft` использует маску `np.where()` над mmap массивом в полгигабайта, извлекая сырые индексы памяти. `inject_graft` записывает максимальную силу веса (32767) по этим оффсетам. Скорость выполнения ограничена только шиной RAM.
