# Структура Проекта Genesis

Rust Workspace с четырьмя крейтами, соответствующими архитектурным слоям из спецификации.

```
Genesis/
├── Cargo.toml                  ← workspace root
├── Cargo.lock
├── config/                     ← TOML конфиги (анатомия, blueprints, сетка)
│   ├── simulation.toml         ← глобальные параметры (voxel_size, sync_batch_ticks, порты)
│   ├── brain.toml              ← зоны, связи, загрузка конфигов
│   ├── anatomy.toml            ← нейронные типы, слои, точки входа
│   ├── blueprints.toml         ← правила спрутинга, пластичность
│   └── zones/                  ← конфиги per-zone
│       ├── _template/
│       ├── SensoryCortex/
│       │   └── io.toml         ← входные/выходные матрицы для этой зоны
│       ├── HiddenCortex/
│       │   └── io.toml
│       └── MotorCortex/
│           └── io.toml
├── scripts/                    ← утилиты для запуска, тестирования, профилирования
│   ├── bake_all.sh             ← вызов baker для всех конфигов
│   └── cartpole_client.py      ← Gym CartPole → наружные I/O UDP
├── baked/                      ← бинарные блобы, сгенерированные baker
│   ├── SensoryCortex/
│   │   ├── shard.state         ← нейронное состояние SoA (soma.flags, voltage, ...)
│   │   ├── shard.axons         ← топология аксонов
│   │   ├── shard.gxi           ← Input Mapping (§08)
│   │   ├── shard.gxo           ← Output Mapping (§08)
│   │   └── SensoryCortex.gxi   ← метаданные входа
│   ├── HiddenCortex/
│   │   ├── shard.state
│   │   ├── shard.axons
│   │   ├── shard.gxo
│   │   └── HiddenCortex_MotorCortex.ghosts ← межзональные связи
│   └── MotorCortex/
│       ├── shard.state
│       ├── shard.axons
│       └── MotorCortex.gxo     ← метаданные выхода
├── docs/
│   ├── specs/                  ← архитектурные спецификации (8 файлов)
│   └── project_structure.md    ← этот файл
│
├── genesis-core/               ← общие типы, константы, SoA layout
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── types.rs            — PackedPosition, type aliases (u32, i16, i32)
│       ├── constants.rs        — AXON_SENTINEL, MAX_DENDRITE_SLOTS, PROPAGATION_LENGTH...
│       └── layout.rs           — SoA структуры, padded_n, Columnar Layout helpers
│
├── genesis-baker/              ← TOML → бинарные блобы (.state, .axons) — CPU only
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── main.rs             — CLI: `baker compile <anatomy.toml> <blueprints.toml>`
│       ├── parser/             — разбор TOML конфигов
│       │   ├── mod.rs
│       │   ├── anatomy.rs      — зоны, нейронные группы, топология
│       │   ├── blueprints.rs   — нейронные типы, параметры, sprouting weights
│       │   ├── io.rs           — чтение/запись файлов
│       │   └── simulation.rs   — параметры симуляции
│       ├── validator/          — проверка инвариантов
│       │   ├── mod.rs
│       │   └── checks.rs       — Sentinel assert, inertia_lut×potentiation >= 1, ...
│       └── bake/               — сборка бинарных блобов
│           ├── mod.rs
│           ├── layout.rs       — SoA упаковка (Columnar, padded_n, warp alignment)
│           ├── sprouting.rs    — compute_power_index, sprouting_score
│           ├── neuron_placement.rs — размещение нейронов в 3D пространстве
│           ├── axon_growth.rs  — рост аксонов, трассировка
│           ├── cone_tracing.rs — конусная трассировка для аксонов
│           ├── dendrite_connect.rs — алгоритм подключения дендритов
│           ├── spatial_grid.rs — пространственная сетка для поиска соседей
│           └── seed.rs         — детерминированный RNG seed
│
├── genesis-runtime/            ← оркестратор + CUDA ядра (Day/Night Cycle)
│   ├── Cargo.toml
│   ├── build.rs                ← сборка CUDA с nvcc
│   ├── cuda/                   ← GPU вычислительные ядра (6 шт в Day Phase)
│   │   ├── bindings.cu         ← CUDA FFI биндинги
│   │   ├── physics.cu          ← UpdateNeurons, PropagateAxons, ApplyGSOP
│   │   ├── apply_spike_batch.cu — ApplySpikeBatch (ghost инъекция)
│   │   ├── inject_inputs.cu    — InjectInputs (внешние I/O, виртуальные аксоны)
│   │   ├── readout.cu          — RecordOutputs (soma → output_history)
│   │   ├── sort_and_prune.cu   — Night Phase: сортировка, удаление неактивных
│   │   └── ghost_sync.cu       — D2H/H2D для межзональных связей
│   │
│   └── src/
│       ├── lib.rs
│       ├── main.rs             — точка входа, инициализация 5 компонентов (§09 3.1)
│       ├── config.rs           — конфигурация runtime
│       ├── memory.rs           — VramState, управление GPU/CPU памятью
│       ├── input.rs            — парсинг GXI (Input Mapping файлов)
│       ├── output.rs           — парсинг GXO (Output Mapping файлов)
│       ├── ffi.rs              — FFI биндинги к CUDA
│       ├── ipc.rs              — IPC протокол с genesis-baker-daemon
│       ├── zone_runtime.rs     — ZoneRuntime lifecycle (Day/Night)
│       ├── orchestrator/       — управление Day/Night Cycle, триггеры сна
│       │   ├── mod.rs
│       │   ├── day_phase.rs    — запуск GPU батчей (6 ядер), BSP барьер
│       │   └── night_phase.rs  — Maintenance Pipeline (Sort, Bake, Upload)
│       └── network/            — IPC, BSP синхронизация, внешние I/O
│           ├── mod.rs
│           ├── bsp.rs          — Strict BSP, сетевой барьер, ping-pong
│           ├── ring_buffer.rs  — SpikeSchedule, Ping-Pong Double Buffering
│           ├── socket.rs       — TCP/UDP транспорт
│           ├── router.rs       — маршрутизация сообщений между зонами
│           ├── intra_gpu.rs    — IntraGpuChannel для ghost синка
│           ├── ghosts.rs       — загрузка и парсинг .ghosts файлов
│           ├── external.rs     — ExternalIoServer (UDP 8081-8082 для хоста)
│           ├── geometry_client.rs — GeometryServer (TCP port+1)
│           └── telemetry.rs    — TelemetryServer (WebSocket port+2 для IDE)
│
└── genesis-ide/                ← Bevy-based IDE для визуализации и управления
    ├── Cargo.toml
    └── src/
        ├── main.rs             — инициализация Bevy App, регистрация плагинов
        ├── loader.rs           — загрузка геометрии нейронов (WebSocket)
        ├── world.rs            — 3D world rendering, генерация Spike Mesh
        ├── camera.rs           — управление камерой (OrbitCamera)
        ├── hud.rs              — HUD оверлей (Egui): статистика, контролы
        └── telemetry.rs        — приём телеметрии из runtime (спайки, фазы)
```

## Зависимости между крейтами

```
genesis-core  ←─── genesis-baker
genesis-core  ←─── genesis-runtime
genesis-core  ←─── genesis-ide
```

`genesis-baker` и `genesis-runtime` не зависят друг от друга.  
Общий контракт данных (блобы `.state` / `.axons`) — файловый обмен.  
`genesis-ide` подключается к `genesis-runtime` по WebSocket для получения геометрии и телеметрии.

## Коммуникация между компонентами

```
genesis-baker  ──[.state/.axons/.gxi/.gxo/.ghosts]──►  genesis-runtime  ──[WebSocket]──►  genesis-ide
                                                               │
                                                        ┌──────┴──────┐
                                                        ▼             ▼
                                                    (External)        (IDE)
                                                      python       visualization
                                                      client
```

| Канал | Протокол | Направление | Данные |
|---|---|---|---|
| baker → runtime (VRAM) | Файлы `.baked/` | одна сторона | .state, .axons, .gxi, .gxo, .ghosts SoA блобы |
| runtime ← external (Input) | UDP 8081 | входящий | ExternalIoHeader + Input_Bitmask батчи |
| runtime → external (Output) | UDP 8082 | исходящий | ExternalIoHeader + Output_History батчи |
| runtime → ide (Geometry) | TCP port+1 | исходящий | 3D позиции нейронов |
| runtime → ide (Telemetry) | WebSocket port+2 | push | TelemetryFrameHeader + spike_ids |

## Соответствие спецификации

| Крейт | Спека |
|---|---|
| `genesis-core` | [07_gpu_runtime.md](./specs/07_gpu_runtime.md) §1 (SoA Layout, VariantParameters) |
| `genesis-baker` | [09_baking_pipeline.md](./specs/09_baking_pipeline.md), [02_configuration.md](./specs/02_configuration.md), [04_connectivity.md](./specs/04_connectivity.md) §1.6.1 |
| `genesis-runtime` | [05_signal_physics.md](./specs/05_signal_physics.md) (6 ядер), [06_distributed.md](./specs/06_distributed.md) (BSP), [07_gpu_runtime.md](./specs/07_gpu_runtime.md) §2-3 (Day/Night) |
| `genesis-ide` | [08_ide.md](./specs/08_ide.md) (Bevy 0.15, WebSocket, 3D world) |

---

## Changelog

**v2.0 (2026-02-28) — Полная синхронизация с реальной структурой**

- **PS.1 (P0)**: Добавлены все недостающие конфиги в `config/` (simulation.toml, brain.toml, zones/*/io.toml)
- **PS.1 (P0)**: Добавлены бинарные файлы в `baked/` (.gxi, .gxo, .ghosts)
- **PS.1 (P0)**: Добавлена папка `scripts/` с утилитами (bake_all.sh, cartpole_client.py)
- **PS.2 (P1)**: Добавлен `cuda/` слой с 6 CUDA ядрами (physics.cu, inject_inputs.cu, readout.cu, sort_and_prune.cu, apply_spike_batch.cu, ghost_sync.cu)
- **PS.2 (P1)**: Добавлены новые runtime модули (input.rs, output.rs, ipc.rs, zone_runtime.rs)
- **PS.3 (P1)**: Обновлена таблица коммуникаций с UDP External I/O (8081 Input, 8082 Output) и TCP/WebSocket сервисами
- Добавлены cross-reference ссылки на спецификации (08_ide.md, 09_baking_pipeline.md)
