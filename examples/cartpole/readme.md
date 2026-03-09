# 🛒 CartPole Genesis Environment (3-Layer Hierarchy)

> **Статус:** ✅ Стабильный (GNM v2)
> **Текущий рекорд:** 🏆 100+ баллов (@H4V1K-dev / Antigravity)
> **Архитектура:** `Sensory (L4) → Hidden (L2/3) → Motor (L5)`

Система использует полноценную трехслойную кортикальную иерархию (450,000+ нейронов). Благодаря промежуточному слою `HiddenCortex` и GNM-пластичности, сеть обучается балансировать маятник, достигая **100 баллов** за ~50-100 эпизодов при TPS **3000+**.

## ВНИМАНИЕ

>После первого запуска у вас будет очень низкий TPS. Это нормально. Сеть должна обучиться и тогда TPS вырастет. Ориентировочно в 40-45 раз за 3-4млн тактов. 

**В клиенте поначалу вы будете часо видеть лог:**
> ⚠️  [Genesis] RX Timeout — waiting for node...
- Это нормально, сеть по началу не оптимизирована и медленно отвечает.
- Эти сообщения пропадут примерно через 100-200 тыс тактов (20-30 минут).
- Затем TPS вырастет и 100_000 тактов будут пролетать за 20-25 секунд (в худшем случае).

### ♻️ Hot-Reload (Живое редактирование)
После запекания в папке `baked/` появятся `manifest.toml` для каждого шарда. Эти файлы можно править **не останавливая** симуляцию.

**Безопасно менять (подхватится за ~0.5 сек):**
- `[settings] night_interval_ticks`: частота нейрогенеза.
- `[settings] save_checkpoints_interval_ticks`: частота автосохранения.
- `[settings.plasticity] prune_threshold`: порог удаления слабых связей.
- `[[variants]]`: любые параметры физики нейронов (threshold, leak_rate, gsop_potentiation и т.д.).

> [!WARNING]
> Остальные параметры (порты, пути, `ghost_capacity`, `padded_n`) требуют перезапуска ноды, так как они определяют структуру памяти в VRAM.

### 🧠 Баланс весов и нейрогенез | Главная задача

**Добиться** баланса 4:1 между возбуждающими (Exc) и тормозными (Inh) синапсами во время обучения — ключ к высокому TPS и стабильному росту.

**SMA-300** - это ваш главный ориентир. Если он падает ниже 100, значит сеть деградирует (обычно из-за доминирования Inh-нейронов или переобучения).
> Показатель успеха это плавный рост SMA-300 до 100 и выше.

- `initial_synapse_weight`: Стартовая сила новых связей (Sprouting). Увеличьте для Exc-нейронов, чтобы дать им "фору" в конкуренции с тормозными путями.
- `gsop_potentiation`: Скорость роста веса при обучении (LTP). Увеличьте для Exc-нейронов, чтобы они быстрее закрепляли полезный опыт при получении дофамина.
- `save_checkpoints_interval_ticks`: частота автосохранения. Уменьшите для более частого сохранения и анализа в самом начале когда TPS низкий. Увеличьте для более редкого сохранения и анализа когда TPS высокий чтобы снизить нагрузку на ваш SSD.

Скрипт анализирует `checkpoint.state` (обновляется каждые `save_checkpoints_interval_ticks` тактов, по умолчанию 10,000) и выводит статистику отклонения от цели:
```bash
python3 scripts/weight_checker.py
```

---

## 🚀 Быстрый старт

### 1. Подготовка окружения
При первом запуске создайте виртуальное окружение:
```bash
python3 -m venv .venv
source .venv/bin/activate
pip install numpy matplotlib gymnasium pygame
```

### 2. Запекание сети (Baking)
Очистка SHM, остановка старых процессов и сборка топологии:
```bash
rm -rf baked/ && \
pkill -f genesis-node; \
pkill -f genesis-baker-daemon; \
rm -f /dev/shm/genesis_shard_* && \
cargo run --release -p genesis-baker --bin baker -- --brain examples/cartpole/config/brain.toml
```

### 3. Запуск Симуляции (3 Шарда)
Запуск всех трех слоев коры в одном процессе (IntraNode):
```bash
cargo run --release -p genesis-node -- \
  --manifest baked/SensoryCortex/manifest.toml \
  --manifest baked/HiddenCortex/manifest.toml \
  --manifest baked/MotorCortex/manifest.toml \
  --batch-size 100 \
  --cpu-profile aggressive
```

### 4. Запуск Клиента и Дашборда
В разных терминалах:
```bash
# Обучение
python3 examples/cartpole/cartpole_client.py

# Мониторинг (SMA-25/100/300, TPS, Скроллинг)
python3 scripts/live_dashboard.py
```

---

## 📊 Аналитика и Визуализация

- **Внутренние веса (Green/Red Map):**
```bash
python3 scripts/visualize_internal_weights.py baked/HiddenCortex/checkpoint.state
```
- **Межзональные связи (3D Ghosts):**
```bash
python3 scripts/visualize_ghosts.py
```
- **Дебаг нейронов:**
```bash
python3 scripts/brain_debugger.py baked/MotorCortex/checkpoint.state
```

---

## 🏆 Hall of Fame

| Участник | Рекорд | Дата | Изменения |
| :--- | :---: | :---: | :--- |
| **@H4V1K-dev** | **100+** | 09.03.2026 | **3-слойная иерархия**, GNM L2/3, Persistence Logic |
| **@shuanat** | **71** | 07.03.2026 | Добавил `random` при равенстве (старая 2-слойка) |

## Удачи в обучении! 🚀
