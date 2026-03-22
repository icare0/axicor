# Brain Builder: HFT-Коннектом и Offline-Компиляция

В Axicor вы не создаете нейроны в оперативной памяти Питона через `new Neuron()`. Это убьет сборщик мусора (GC) и сделает невозможным Zero-Copy загрузку на GPU. 

Вместо этого мы используем паттерн **"ДНК Мозга"**. С помощью Python-класса `BrainBuilder` вы генерируете строгую топологию в виде TOML-конфигов. Затем утилита `genesis-baker` "запекает" эту ДНК в плоские бинарные C-ABI массивы (`.state` и `.axons`). Это основа движка Axicor.

## 1. Жизненный цикл (Pipeline Clarity)

1. **Python Script (`builder.build()`):** Генерирует иерархию папок и TOML-файлов (`anatomy.toml`, `blueprints.toml`, `io.toml`, `shard.toml`).
2. **Genesis Baker (CPU Compiler):** `cargo run -p genesis-baker -- --brain config/brain.toml`. Читает TOML, расставляет нейроны, "проращивает" аксоны через 3D-пространство (Cone Tracing) и сохраняет выровненные по варпам VRAM-дампы.
3. **Genesis Node (GPU Runtime):** `cargo run -p genesis-node`. Делает `mmap` запеченных дампов в память ОС напрямую, минуя аллокации.

---

## 2. Создание HFT-Коннектома (Эталон)

При работе в высокочастотном цикле (100+ Гц) со средами вроде Gymnasium, базовая пластичность из биологической библиотеки убьет веса. Мы обязаны глушить фоновый рост и переводить сеть в режим **Reward-Gated Plasticity** (рост только при дофамине).

```python
from genesis.builder import BrainBuilder

# 1. Инициализация архитектора
builder = BrainBuilder(project_name="HftAgent", output_dir="Genesis-Models/")

# Тонкая настройка физики под HFT-цикл (1 ms барьер)
builder.sim_params["sync_batch_ticks"] = 10
builder.sim_params["tick_duration_us"] = 100

# 2. Создание зоны (Ширина, Глубина, Высота в вокселях)
# Размер вокселя по умолчанию = 25 мкм
cortex = builder.add_zone("SensoryCortex", width_vox=64, depth_vox=64, height_vox=63)

# 3. Загрузка типов и HFT-тюнинг пластичности
# В HFT-режиме рост весов происходит ТОЛЬКО при вливании дофамина (pot=0)
# Фоновая депрессия (dep=2) медленно выжигает случайный шум. Это важно для Axicor.
exc_type = builder.gnm_lib("VISp4/141").set_plasticity(pot=0, dep=2)
inh_type = builder.gnm_lib("VISp4/114").set_plasticity(pot=0, dep=2)

# 4. Наполнение слоями (Bottom-Up дизайн)
# ВНИМАНИЕ: height_pct слоев в сумме должны давать строго 1.0!
cortex.add_layer("L4_Input", height_pct=0.1, density=0.3)\
      .add_population(exc_type, fraction=0.8)\
      .add_population(inh_type, fraction=0.2)

cortex.add_layer("L23_Hidden", height_pct=0.6, density=0.15)\
      .add_population(exc_type, fraction=0.8)\
      .add_population(inh_type, fraction=0.2)

cortex.add_layer("L5_Motor", height_pct=0.3, density=0.1)\
      .add_population(exc_type, fraction=1.0)

# 5. Определение Входов и Выходов (I/O Matrix)
cortex.add_input("sensors", width=8, height=8, entry_z="top")
cortex.add_output("motors", width=16, height=8, target_type="All")

# 6. Компиляция ДНК
builder.build()
```

## 3. Как это работает под капотом (DOD)

1. **Никакого RNG для популяций (Hard Quotas):** Метод `add_population(fraction=0.8)` устанавливает жесткую квоту. Если в слое помещается 1000 нейронов по объему и density, движок гарантированно создаст ровно 800 нейронов одного типа и 200 другого. Типы перемешиваются детерминированным шаффлом (master_seed).
2. **Матричный маппинг:** Входы и выходы (`add_input`, `add_output`) растягиваются на 3D-сетку зоны (UV-проекция). Один пиксель матрицы охватывает популяцию физических нейронов. Выбор конкретной сомы для I/O детерминирован алгоритмом Spatial Hashing.
3. **C-ABI выравнивание:** Baker автоматически вычисляет `padded_n` и добивает количество нейронов нулями до числа, кратного 32 (Warp Alignment). GPU будет читать эту память без Divergence и Cache Misses.
4. **Закон Дейла (Strict Dale's Law):** В Axicor знак веса синапса (возбуждающий или тормозной) — это не параметр самого синапса или дендрита. Знак определяется **исключительно типом пресинаптического нейрона (источника)**. Если тип помечен как `is_inhibitory = true` в `blueprints.toml`, все его аксоны будут нести только тормозные сигналы. Это решение «цементируется» Baker-ом при генерации бинарных дампов. Такое ограничение позволяет GPU-ядрам проводить GSOP-пластичность без единого ветвления (Branchless Math), что критично для HFT-цикла.

## 4. Shift-Left Validation & Ergonomics

Python SDK (`BrainBuilder`) выступает в роли первого эшелона защиты C-ABI контрактов. Валидация должна происходить до генерации `.toml` файлов.

### 4.1. Interactive Auto-Fix
Если скрипт запущен в интерактивном терминале (`sys.stdout.isatty()`), при обнаружении архитектурного нарушения SDK обязан остановить выполнение, показать текущее ошибочное значение, математически рассчитать ближайшие валидные варианты и предложить пользователю выбор (ввод числа или Enter для дефолтного автофикса).
В неинтерактивных средах (CI/CD) скрипт должен падать с `ValueError`.

### 4.2. Integer Physics Validation (`v_seg`)
Скорость распространения сигнала обязана быть кратна длине сегмента.
*   **Правило:** `v_seg = (signal_speed_m_s * 1000 * (tick_duration_us / 1000)) / (voxel_size_um * segment_length_voxels)`. Значение `v_seg` должно быть строго целым.
*   **UX:** При дробном `v_seg` SDK предлагает изменить либо `signal_speed_m_s`, либо `segment_length_voxels`, отображая точные рассчитанные значения.

### 4.3. Topological Auto-Routing (MTU Fragmentation)
Пользователь оперирует только логическими размерами матриц (например, `width=256, height=256`).
*   **Правило:** SDK вычисляет размер payload'а: `(width * height) / 8` байт.
*   **Фрагментация:** Если payload превышает заданный MTU (по умолчанию 65507 для PC, 1400 для ESP32), `BrainBuilder` автоматически разбивает логическую матрицу на `N` физических подматриц (чанков).
*   **Маппинг:** Каждому чанку автоматически назначается пространственный оффсет `uv_rect = [u_offset, v_offset, u_width, v_height]`, чтобы `genesis-baker` собрал их в единую сетку без перекрытий.

### 4.4. 4-Bit Type Limit
Внутри одной зоны (Shard) может быть не более 16 уникальных типов нейронов, так как `type_mask` занимает строго 4 бита в массиве `soma_flags`. Попытка добавить 17-й тип через `add_population` мгновенно прерывается ошибкой.

## 5. Определение Входов и Выходов (I/O Matrix & Blueprint Wiring)
Входы и выходы привязываются к физическим слоям зоны. Для избежания ООП-оверхеда на стороне клиента, мы используем семантическую разметку `layout`.

```python
# Матрица 8x8 (64 виртуальных аксона)
# Первые 3 слота привязываются к читаемым именам, остальные 61 добиваются нулями (Padding)
cortex.add_input("sensors", width=8, height=8, entry_z="top", layout=[
    "pos_x", "pos_y", "angle_joint_0"
])

# Выходная матрица 16x8
cortex.add_output("motors", width=16, height=8, target_type="All", layout=[
    "motor_left", "motor_right"
])
```

Физика layout: Rust-компилятор (Data Plane) игнорирует эти строки. Он берет width=8, height=8 и аппаратно аллоцирует ровно 64 слота, выровненных по границе кэш-линии (The 64-Byte Alignment Rule). Python SDK (Control Plane) читает эти строки из io.toml и динамически генерирует Zero-Cost Фасад, где pos_x — это прямое замыкание на memoryview, а pos_y на memoryview.

