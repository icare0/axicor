# 05. Физика Сигнала и I/O (Signal Physics)

> Часть архитектуры [Genesis](../../design_specs.md). Полный путь сигнала: вход → распространение → выход.

---

## 1. Физика Сигнала (Signal Physics)

Движение сигнала и реакция синапсов реализуются через целочисленную арифметику индексов. Логика оптимизирована для минимизации чтений глобальной памяти (Global Memory Reads).

### 1.1. Движение: «Поезд» (The Train Model)

Сигнал — не точка, а кортеж активных индексов.

- **Состояние аксона:** Хранится только `Head_Index` (`u32`).
- **Константа:** `v_seg` (скорость в сегментах/тик) — рассчитывается при Baking.
- **Update Loop:** Каждый тик: `Head_Index += v_seg`.
- **Active Tail:** Сегмент `i` считается активным, если:

```
Head - Tail_Length <= i <= Head
```

> **Инвариант:** `signal_propagation_length ≥ v_seg`.
> Иначе при `v_seg > propagation_length` между тиками возникают "прыжки", когда сегменты
> проскакивают мимо хвоста и никогда не попадают в Active Tail — дендриты к ним мертвы.

Это гарантирует, что даже при высокой скорости «поезд» не перепрыгнет дендрит за один тик.

### 1.2. Инференс: Фильтрация (Inference Pipeline)

Выполняется каждый тик для каждого дендрита. Стратегия **Early Exit** для разгрузки шины памяти.

**Шаг 1. Refractory Gate (Отсечение на первом чтении):**

```cuda
// Refractory timer — 1 байт на дендрит. 32 потока × 1 байт = 32 байта (1 сектор L1)
u8 timer = refractory_timer[slot * N_padded + tid];
if (timer > 0) {
    refractory_timer[slot * N_padded + tid] = timer - 1;
    return; // Early Exit: ~90% тиков дендрит «спит» → НЕ читаем Global Memory аксона
}
```

**Шаг 2. Overlap Check (Проверка перекрытия):**

- Читаем `Axon_Head_Index` из глобального массива аксонов.
- Проверяем: попадает ли подключённый сегмент в интервал Active Tail.
- Если **да:**
  - `Soma_Voltage += Synapse_Weight` (атомарное сложение или агрегация в warp).
  - `dendrite.timer = const_mem.variants[variant_id].synapse_refractory` (значение из Constant Memory, 1 такт).

### 1.2.1. Инъекция Сетевых Спайков: ApplySpikeBatch

Ядро #2 Day Phase. Принимает массив `ghost_indices[]` из `SpikeBatch` (см. [06_distributed.md §2.5](./06_distributed.md)). Sender-Side Mapping: индексы уже готовые `u32`, никакого перевода ID.

> **Early Exit:** Если `count == 0` (нет спайков от соседей в этом тике), ядро мгновенно завершается — ноль математики, ноль транзакций памяти.

```cuda
__global__ void ApplySpikeBatch(u32* axon_heads, const u32* ghost_indices, u32 count) {
    u32 tid = blockIdx.x * blockDim.x + threadIdx.x;
    if (tid >= count) return;

    // O(1) routing. ghost_indices[tid] = абсолютный индекс в axon_heads[].
    // Bounds checking (index < total_axons) гарантированно выполнен на CPU 
    // во время Map Phase (06_distributed.md §2.8).
    // Сброс Sentinel → 0 = рождение сигнала.
    axon_heads[ghost_indices[tid]] = 0;
}
```

Обучение запускается **только** когда сома генерирует спайк (`Is_Spiking == true`). Вместо анализа истории («кто стрелял 5 мс назад?»), анализируем **текущее пространственное состояние** Active Tail.

- **Принцип:** Сома стреляет → хвост аксона всё ещё касается дендрита → значит, этот аксон участвовал в возбуждении. Причинно-следственная связь через перекрытие, не через временные метки.

**Constant Memory:** `GenesisConstantMemory` (см. [07_gpu_runtime.md §1.5](./07_gpu_runtime.md)).

### 1.3. Инференс: Пластичность (ApplyGSOP)

**Kernel ApplyGSOP (100% Integer, 0% Float, Branchless):**

```cuda
__global__ void ApplyGSOP(u8* flags, u32* dendrite_target, i16* dendrite_weights,
                          u8* dendrite_timers, u32 N) {
    u32 tid = blockIdx.x * blockDim.x + threadIdx.x;

    // 1. Early Exit: 99.9% потоков уходят здесь (спайк ~1-10 Hz)
    if (!(flags[tid] & 0x1)) return;

    // 2. Загрузка параметров через struct (L1 Cache, 1 такт)
    u8 var_id = (flags[tid] >> 6) & 0x3;   // Биты 6-7 = Variant
    VariantParameters p = CONST_MEM.variants[var_id];

    // 3. Columnar Loop: 128 слотов (Coalesced Access)
    #pragma unroll
    for (int slot = 0; slot < 128; ++slot) {
        u32 col_idx = slot * N + tid;

        u32 target_packed = dendrite_target[col_idx];
        if (target_packed == 0) break; // Columnar Defrag invariant: first empty = all empty

        // 4. Causal Check: Timer-as-Contact-Flag
        // UpdateNeurons (Step 4) уже записал результат в dendrite_timers:
        //   timer == synapse_refractory → контакт был в этом тике (Potentiation)
        //   timer == 0                  → контакта не было (Depression)
        // Никаких повторных чтений axon_heads. Нет race condition.
        u8 d_timer = dendrite_timers[col_idx];
        u32 is_causal = (d_timer == p.synapse_refractory);

        // 5. Inertia Rank: abs(weight) >> 11 → 16 рангов (по 2048 единиц)
        i16 w = dendrite_weights[col_idx];
        u16 abs_w = (u16)abs(w);
        u8 inertia = p.inertia_curve[abs_w >> 11];

        // 6. Branchless GSOP Math (Zero Float)
        u16 delta_pot = (p.gsop_potentiation * inertia) >> 7;
        u16 delta_dep = (p.gsop_depression * inertia) >> 7;
        i32 delta = is_causal * delta_pot - (!is_causal) * delta_dep;

        // 7. Slot Decay: LTM/WM множители из конфига (Fixed-point: 128 = 1.0×)
        u8 decay = (slot < p.ltm_slot_count) ? p.slot_decay_ltm : p.slot_decay_wm;
        delta = (delta * decay) >> 7;

        // 8. Signed Clamp ±32767 (Branchless sign extraction)
        i32 w_sign = ((i32)w >> 31) | 1;    // +1 or -1
        i32 new_abs = (i32)abs_w + delta;
        dendrite_weights[col_idx] = (i16)(w_sign * max(0, min(32767, new_abs)));
    }
}
```

**Pruning:** Если `abs(Weight) < Prune_Threshold` после обновления, слот помечается как свободный (`target_packed = 0`) для Sprouting в фазу «Ночь».

### 1.4. Почему это эффективно

| Принцип | Эффект |
|---|---|
| **Memory Bandwidth** | Чтение состояния чужих аксонов только когда рефрактерность закончилась. В ~90% тиков дендрит «спит» — шина не нагружается. |
| **No History Buffers** | Не храним `last_spike_time` для миллиардов синапсов. Экономия ~4–8 ГБ VRAM. |
| **Warp Divergence** | Все ветки `if/else` минимизированы. GSOP (самая ветвистая часть) выполняется редко — только при спайке сомы. |

### 1.5. Главный Тик: UpdateNeurons (GLIF Kernel)

Ядро, которое собирает всю физику в один проход: GLIF leak, гомеостаз, Early Exit, суммация дендритов, threshold check, fire/reset. Параметры читаются из `GenesisConstantMemory` (см. [07_gpu_runtime.md §1.5](./07_gpu_runtime.md)).

```cuda
__constant__ GenesisConstantMemory CONST_MEM;

__global__ void UpdateNeurons(
    u8* flags, i32* voltage, u8* soma_ref_timer, i32* threshold_offset,
    u32* dendrite_target, i16* dendrite_weights, u8* dendrite_timers,
    u32* axon_heads, u32* soma_to_axon, u32 N
) {
    u32 tid = blockIdx.x * blockDim.x + threadIdx.x;

    // 1. Распаковка типа + загрузка параметров (1 такт L1)
    u8 f = flags[tid];
    u8 var_id = (f >> 4) & 0xF;              // Биты 4-7 = Variant (16 типов)
    VariantParameters p = CONST_MEM.variants[var_id];

    // 2. Гомеостаз (Soft Limit) — выполняется ВСЕГДА, даже когда сома спит
    i32 t_off = threshold_offset[tid];
    i32 decayed = t_off - p.homeostasis_decay;
    t_off = decayed & ~(decayed >> 31);       // Branchless max(0, val)

    // 3. Рефрактерность сомы — Early Exit (~90% потоков)
    u8 s_ref = soma_ref_timer[tid];
    if (s_ref > 0) {
        soma_ref_timer[tid] = s_ref - 1;
        threshold_offset[tid] = t_off;
        flags[tid] = f & ~0x1;
        return;
    }

    // 4. GLIF: Утечка заряда (Branchless clamp ≥ rest)
    i32 v = voltage[tid];
    i32 leaked = v - p.leak;
    i32 diff = leaked - p.rest_potential;
    v = p.rest_potential + (diff & ~(diff >> 31));  // max(rest, leaked)

    // 5. Columnar Loop: 128 дендритных слотов (Coalesced Access)
    #pragma unroll
    for (int slot = 0; slot < 128; ++slot) {
        u32 col_idx = slot * N + tid;

        // 5a. Refractory Gate (Early Exit — экономим 3 global reads)
        u8 d_ref = dendrite_timers[col_idx];
        if (d_ref > 0) {
            dendrite_timers[col_idx] = d_ref - 1;
            continue;  // ~90% тиков: дендрит спит → skip target/head/weight
        }

        // 5b. Пустой слот — BREAK, не continue!
        // Инвариант: после Night Phase (Baking §4.2 Columnar Defrag) все пустые
        // слоты (target=0) гарантированно в хвосте массива. target_packed
        // заморожен на GPU весь Day Phase — никакое ядро его не пишет.
        // => Первый target==0 означает: все оставшиеся слоты тоже пусты.
        u32 target = dendrite_target[col_idx];
        if (target == 0) break;  // Early exit: O(K) вместо O(128), K = кол-во живых слотов

        // 5c. Active Tail Overlap Check (u32 overflow легализован)
        u32 axon_id = target >> 10;
        u32 seg_idx = target & 0x3FF;
        u32 head    = axon_heads[axon_id];
        u32 dist    = head - seg_idx;

        if (dist <= p.propagation_length) {
            // 5d. Voltage accumulation (i16→i32, знак запечён)
            i16 w = dendrite_weights[col_idx];
            v += (i32)w;

            // 5e. Timer reset из Constant Memory (1 такт)
            dendrite_timers[col_idx] = p.synapse_refractory;
        }
    }

    // 6. Threshold Check & Fire (Branchless)
    i32 eff_threshold = p.threshold + t_off;
    u32 is_spiking = (v >= eff_threshold);

    v     = is_spiking * p.rest_potential + (1 - is_spiking) * v;
    s_ref = is_spiking * p.refractory_period;
    t_off += is_spiking * p.homeostasis_penalty;
    f     = (f & 0xFE) | (u8)is_spiking;

    // 7. Сброс аксона при спайке (Predicated Store: @p st.global.u32)
    if (is_spiking) {
        axon_heads[soma_to_axon[tid]] = 0;
    }

    // 8. Запись в VRAM
    voltage[tid] = v;
    soma_ref_timer[tid] = s_ref;
    threshold_offset[tid] = t_off;
    flags[tid] = f;
}
```

### 1.6. Пропагация Сигнала: PropagateAxons

Безусловный сдвиг **всех** аксонов (Local + Ghost + Virtual). Количество аксонов `A ≠ N` (сом). Запускается каждый тик **до** `UpdateNeurons`.

**Sentinel:** `AXON_SENTINEL = 0x80000000`. При инициализации и после Night Phase все неактивные аксоны инициализируются этим значением. `dist = 0x80000000 - seg_idx` = огромное число → `is_active = false`. Overflow до 0 произойдёт через ~59.6 часов.

> **⚠️ Sentinel Refresh:** Для зон с `night_interval_ticks = 0` (или редким сном) host каждые ~50 часов сбрасывает переполненные `axon_heads` обратно в `AXON_SENTINEL`. Подробности: [07_gpu_runtime.md §2.2](./07_gpu_runtime.md).

```cuda
#define AXON_SENTINEL 0x80000000u

__global__ void PropagateAxons(u32* axon_heads, u32 total_axons, u32 v_seg) {
    u32 tid = blockIdx.x * blockDim.x + threadIdx.x;
    if (tid >= total_axons) return;

    // 100% Coalesced Access. 1 такт IADD. 0 ветвлений.
    u32 head = axon_heads[tid];
    if (head != AXON_SENTINEL) {
        axon_heads[tid] = head + v_seg;
    }
}
```

- **Рождение сигнала = сброс `axon_heads[id] = 0`:**
  - Локальный спайк: `UpdateNeurons` (шаг 7, predicated store)
  - Сетевой спайк: `ApplySpikeBatch` (Ghost аксоны)
  - Внешний стимул: `InjectInputs` (виртуальные аксоны)
- **1 спайк в полёте:** `signal_propagation_length < soma_refractory_period` → первый поезд успевает доехать до конца до рождения второго.

---

## 2. Входные Интерфейсы (Input Interfaces)

Входы реализуются как **Виртуальные Аксоны** (Virtual Axons). Конфигурация задаётся на уровне ноды (`io.toml`), т.к. зоны могут быть распределены по разным машинам.

### 2.1. Топология: Spatial Input Maps

Входной сигнал — не сырая картинка, а результат препроцессинга (Feature Extraction). Каждый вход описывается как **Input Map** — 2D-матрица пикселей, каждый из которых представлен виртуальным аксоном.

- **Множественные маппинги:** На одну зону может приходить N маппингов разных размеров (64×64, 32×32, 128×16). Каждый — отдельный сенсорный канал.
- **target_type:** Каждый маппинг привязан к типу нейрона из `blueprints.toml` (или `"ALL"` без ограничений). Baker ищет сомы этого типа для размещения.

#### Baking: Размещение через Seeded Hash

Для каждого пикселя каждого маппинга:

1. Вычислить пространственный регион: `region_xy = pixel_xy * (zone_size / map_size)`. Все Z внутри региона.
2. Собрать кандидатов (сомы `target_type` в регионе).
3. Выбрать сому: `seed = master_seed ^ fnv1a(input_name) ^ pixel_idx`, `chosen = candidates[hash(seed) % len]`.
4. Если нужен retry (вторичная коллизия): `seed' = ... ^ (pixel_idx + attempt)`.
5. **Вырастить** виртуальный аксон с параметрами `target_type` (velocity, propagation_length, steering). Аксон прорастает от вокселя сомы, создавая пятно влияния — не точечный 1-сегментный стаб.
6. Проверки коллизий вокселей **не требуется**: хеш разводит пиксели одной матрицы в разные регионы, а разные матрицы дают разный seed (из-за имени).

#### Multi-Shard

- Каждый шард получает свой файл маппинга `{shard_name}.gxi`.
- Baker определяет шард по координатам региона.
- GhostPacket при прорастании через границу шарда (штатный механизм).
- Host при подаче батча режет матрицы и адресует каждому шарду свой кусок.

### 2.2. Протокол Входа: Single-Tick Pulse

Виртуальные аксоны управляются драйвером через bitmask. Протокол:

- **Пульс:** Драйвер устанавливает бит = 1 на **ровно 1 тик**. `InjectInputs` сбрасывает `axon_heads[id] = 0` (рождение сигнала).
- **Virtual Refractory:** Минимум `signal_propagation_length` тиков между пульсами на один канал. **Ответственность хоста** — runtime не проверяет.
- **Регуляция:** Защита от перевозбуждения лежит на принимающей стороне (дендритная рефрактерность, §1.2).

> **⚠️ Нельзя удерживать бит = 1 на несколько тиков!** InjectInputs каждый тик сбрасывает head = 0, не давая сигналу пропагироваться (голова застревает на сегменте 0-1).

### 2.3. Пирог Признаков (Feature Pyramid Batching)

Видеокадр (например, 30 fps) раскладывается препроцессором на **100+ матриц признаков** (edges, color, motion, corners, ...). Каждая матрица — отдельный слой «пирога». Батч подаётся покадрово: **1 слой = 1 тик**.

```
Кадр 30fps → препроцессор → 100+ масок
  batch[0]  = edges_mask         (тик 0)
  batch[1]  = color_mask         (тик 1)
  batch[2]  = motion_left_mask   (тик 2)
  ...
  batch[99] = corners_mask       (тик 99)
```

Преимущество: мозг получает информацию **постепенно**, а не одним взрывом. Между батчами — тишина (или следующий кадр). Размер батча и периодичность — на усмотрение внешнего интерфейса.

Host загружает весь батч одним DMA. Runtime каждый тик берёт `batch[tick_in_batch]`.

### 2.4. Драйвер Ввода: DMA Bitmask Injection

**Host (CPU):** Формирует плотную битовую маску `Input_Bitmask` (1 бит = 1 виртуальный аксон). Все маппинги конкатенированы в один flat массив.

**Transfer:** `cudaMemcpyAsync(Input_Bitmask_GPU, Input_Bitmask_Host, size, stream)` — микросекунды.

**Kernel (InjectInputs):**

```cuda
__global__ void InjectInputs(u32* axon_heads, const u32* input_bitmask,
                             const u32* map_pixel_to_axon,
                             u32 num_pixels) {
    u32 tid = blockIdx.x * blockDim.x + threadIdx.x;
    if (tid >= num_pixels) return;

    u32 mask = input_bitmask[tid / 32];
    u32 is_active = (mask >> (tid % 32)) & 1;

    if (is_active) {
        axon_heads[map_pixel_to_axon[tid]] = 0;  // Рождение сигнала
    }
}
```

---

## 3. Система Вывода (Readout Interface)

Вывод реализуется как **Direct Memory Access** к состоянию конкретных сом. Формируем временнýю матрицу активности.

### 3.1. Топология: Проекция «Победитель получает всё»

В конфиге `io.toml` определяется зона среза (Volume Slice). Строгое требование уникальности.

**Baking Phase (Compiler Tool):**

1. Зона разбивается на 2D-сетку (каналы вывода).
2. В каждой ячейке сетки ищутся все попадающие в неё сомы (вертикальный столб).
3. **Разрешение коллизий (Z-Sort):**
   - Сортируем кандидатов по высоте Z.
   - Выбираем одного с **наименьшим Z** (ближайшего к дну слоя / Z0).
   - Остальные игнорируются (они работают как часть сети, но к выходу не подключены).

**Результат:** Статический массив `Map_Channel_to_SomaID[]`. Размер = число пикселей выходной матрицы. Маппинг 1-к-1, без конфликтов.

### 3.2. Рантайм: Накопление (Batch Accumulation)

Мы не останавливаем симуляцию каждый тик ради вывода. Данные накапливаются в VRAM.

- **Буфер:** `Output_History[Batch_Size × Num_Channels]` (`u8`) — выделяется при старте.
- **Kernel RecordOutputs (каждый тик, после ApplyGSOP — Kernel #6):**

```cuda
__global__ void RecordOutputs(const u8* flags, const u32* map_channel_to_soma,
                              u8* output_history, u32 num_channels,
                              u32 current_tick_in_batch) {
    u32 ch = blockIdx.x * blockDim.x + threadIdx.x;
    if (ch >= num_channels) return;

    u32 soma_id = map_channel_to_soma[ch];   // O(1), baked
    u8 is_spiking = flags[soma_id] & 0x01;

    // u8 per channel: Coalesced Write, zero atomics
    output_history[current_tick_in_batch * num_channels + ch] = is_spiking;
}
```

- **Почему u8, а не биты?** Bit-packing потребовал бы `atomicOr` при записи — медленно. `u8` на канал = 32 потока варпа пишут 32 непрерывных байта за 1 транзакцию.
- **Маппинг** строго 1-к-1 (Z-Sort из §3.1). Конфликтов записи нет.

### 3.3. Синхронизация: Выгрузка (Flush)

В конце цикла `sync_batch` (вместе с обменом граничными аксонами):

1. **Transfer:** Единый `cudaMemcpy` копирует весь `Output_History` в RAM хоста.
2. **Latency:** Внешний мир получает данные с задержкой в размер батча (например, 10 мс).
   - **Биологическая валидность:** Сигнал от моторной коры до мышцы идёт 10–20 мс. Эта задержка естественна.

### 3.4. Интерпретатор: Population Coding (External Hub)

На выходе Hub получает матрицу `output_history[Time × Channels]` (`u8`: 0 или 1).

**Принцип: Population Coding (не Rate Coding / ШИМ).**

Для каждого действия выделяется **популяция** выходных нейронов. Сила действия пропорциональна количеству одновременно активных нейронов в популяции. Мозг сам учится через GSOP рекрутировать нужное количество.

| Пример | Популяция | Сила |
|---|---|---|
| Сгибание руки | 100 нейронов → 23 активны | `23 / 100 = 0.23` |
| Разгибание руки | 100 нейронов → 87 активны | `87 / 100 = 0.87` |

**Интерпретация на Hub:**

```
// На микроконтроллере (ESP32 / Teensy / STM32):
let active = popcount(population_mask);   // 1 инструкция
let strength = active as f32 / population_size as f32;  // 0.0 .. 1.0
set_motor_output(strength);               // Прямой аналоговый выход
```

- **Мгновенный отклик:** Не нужно ждать N тиков для подсчёта частоты — `popcount` в текущем кадре даёт силу.
- **Биологичность:** В природе сила мышцы = количество рекрутированных моторных единиц.
- **Замкнутая петля:** Моторный выход → робот двинул рукой → датчики изменились → Input обновился → аксоны в L4 выстрелили.
- **Универсальность:** Аудио, моторика, любой выход — одинаковый формат (битовая маска популяции).

### 3.5. Moving Window: Управляемая Дискретность (Control FPS)

`output_history` — двумерная матрица `[sync_batch_ticks × num_channels]`. Hub **не обязан** читать весь батч одним куском.

**Протокол скользящего окна:**

```
window_size = tick_duration_us × sync_batch_ticks / (1_000_000 / target_fps)

// Пример: tick=100мкс, batch=100тиков → батч = 10 мс
// target_fps = 60 → window = 10.000мкс / 16.667мкс ≈ 6 тиков / кадр
// target_fps = 120 → window = 10.000мкс / 8.333мкс ≈ 1-2 тика / кадр
```

Hub читает `output_history` строка за строкой с шагом `window_size` и выдаёт команду приводу с частотой `target_fps`. Настраивается в `io.toml`.

> **Антагонисты (Biceps / Triceps и т.п.):** Разрешение конфликтующих сигналов — ответственность Hub-интерпретатора, не Genesis-движка. Пример: `signal = Strength(A) - Strength(B)` → ненулевой дифференциал = направление и сила. Мозг обучится через GSOP формировать корректные корреляции самостоятельно.

