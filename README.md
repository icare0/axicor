<p align="center">
  <img src="Logo.PNG" alt="Axicor" width="600"/>
</p>

<h1 align="center">Axicor Alpha 0.0.1</h1>

<h3 align="center">Живой мозг для роботов. Учится за секунды. Работает везде — от ESP32 до кластера.</h3>

<p align="center">
  <a href="./docs/specs/">Спецификация</a> · <a href="./CHANGELOG.md">Changelog</a> · <a href="https://t.me/+zptNJAJhDe41ZTEy">Telegram</a>
</p>

<p align="center">
  <a href="https://github.com/H4V1K-dev/Axicor"><img src="https://img.shields.io/badge/status-pre--alpha-orange?style=flat" alt="Status"></a>
  <a href="https://github.com/H4V1K-dev/Axicor"><img src="https://img.shields.io/github/languages/top/H4V1K-dev/Axicor?style=flat&color=dea584" alt="Top Language"></a>
  <a href="https://github.com/H4V1K-dev/Axicor"><img src="https://img.shields.io/github/last-commit/H4V1K-dev/Axicor?style=flat" alt="Last Commit"></a>
  <a href="./LICENSE"><img src="https://img.shields.io/badge/license-GPLv3-blue?style=flat" alt="License"></a>
  <a href="https://ko-fi.com/axicor"><img src="https://img.shields.io/badge/Support-Ko--fi-FF5E5B?style=flat&logo=ko-fi&logoColor=white" alt="Ko-fi"></a>
</p>

## ⚡️ Установка в одну строку

```bash
# Клонировать и настроить окружение (Linux/macOS)
git clone https://github.com/H4V1K-dev/Axicor.git && cd Axicor && ./scripts/setup.sh
```

> **Требуется:** Rust ≥ 1.75, CUDA Toolkit ≥ 12.0, Python ≥ 3.10

---

## Что это такое

Axicor — это движок для биологически-правдоподобных нейронных сетей. Не очередной ML-фреймворк поверх тензоров, а физический симулятор нейронов с настоящей структурной пластичностью.

**Главное отличие от PyTorch, JAX и прочих:**

Обычные нейросети учатся через градиентный спуск — математическую оптимизацию, которая требует остановки и прохода назад по графу. Axicor учится как мозг: нейроны стреляют спайками, аксоны физически прорастают к соседям, слабые связи обрезаются, сильные укрепляются — всё это происходит в реальном времени, без остановки симуляции.

Результат: агент начинает обучаться с первого же действия, а не после тысяч эпизодов прогрева.

---

## Почему это работает быстрее

Три инженерных решения которые отличают Axicor от академических поделок:

**Integer Physics** — вся математика нейронов в целых числах (`u8`, `u16`, `i32`). Никаких `float`. Это даёт абсолютный детерминизм и одинаковое поведение на любом железе — от RTX 4090 до микроконтроллера за $5.

**Day/Night Cycle** — GPU занимается только физикой спайков (Day Phase), CPU в это время перестраивает топологию: обрезает слабые связи, проращивает новые аксоны (Night Phase). Два процесса никогда не конкурируют за ресурсы.

**Structure of Arrays** — данные всех нейронов хранятся в плоских массивах, а не объектах. GPU читает их 100% эффективно, без единого cache miss.

---

## 🚀 Быстрый Старт

Запустите обучающегося агента за четыре команды и смотрите как муравей учится двигаться.

**1. Активируй окружение**
```bash
source .venv/bin/activate
```

**2. Запеки мозг**  
Компилятор читает TOML-чертёж и генерирует бинарный граф нейронов:
```bash
python3 examples/ant_exp/build_brain.py
```

**3. Запусти движок**
```bash
cargo run --release -p genesis-node -- --brain AntAgent
```

**4. Подключи агента**  
В новом терминале — агент начнёт обучаться сразу:
```bash
python3 examples/ant_exp/ant_agent.py
```

Ты увидишь как Score растёт, синапсы прунятся и сеть адаптируется в реальном времени.

---

## Где это можно применить

**Робототехника** — управление сервоприводами и балансировка без облака. Мозг живёт прямо в устройстве.

**Микроконтроллеры** — `genesis-lite` работает на ESP32-S3 ($5) с полным Day/Night циклом. Альтернатива PID-регулятору которая сама адаптируется под изменяющиеся условия.

**Кластеры** — мозг можно физически разрезать на шарды и разнести по разным машинам. Аксоны прорастают между нодами по сети через TCP во время Night Phase. Протестировано на 1.5M нейронов / 150M синапсов на двух физических узлах.

**Нейронаука** — движок может принимать реальные коннектомы: FlyWire Drosophila, Allen Cell Types Database. Можно симулировать реальные нейронные цепи.

---

## Архитектура (для тех кто хочет глубже)

Axicor — это семь компонентов:

| Компонент | Роль | Готовность |
| :--- | :--- | :--- |
| **`genesis-core`** | Общие типы, константы, контракты IPC | ✅ |
| **`genesis-baker`** | Компилятор TOML → бинарные `.state`/`.axons` для GPU | ✅ |
| **`genesis-compute`** | CUDA/ROCm ядра, управление VRAM | ✅ |
| **`genesis-node`** | Оркестратор: BSP барьер, UDP, Night Phase | ✅ |
| **`genesis-client`** | Python SDK: Builder DSL, Zero-Copy Data Plane, Auto-Tuner | ✅ |
| **`genesis-ide`** | 3D визуализатор на Bevy (live спайки) | 🔨 |
| **`genesis-retina`** | Интерфейс машинного зрения | 🔨 |

Полная техническая спецификация в [`docs/specs/`](./docs/specs/) (14 документов) — математика, форматы бинарных файлов и контракты IPC.

---

## Статус

**Pre-alpha. Активная разработка.**

- ✅ 1.5M нейронов на двух физических узлах
- ✅ CartPole E2E (Python Gymnasium → UDP → Axicor → мотор)  
- ✅ Ghost Axon Handover (аксоны прорастают между нодами)
- ✅ ESP32-S3 порт (`genesis-lite`)
- ✅ Bayesian parameter search (Optuna интеграция)
- 🔨 Стабилизация MVP

---

## Contributing

Читай [`CONTRIBUTING.md`](./CONTRIBUTING.md). Если хочешь — заходи в [Telegram-группу разработчиков](https://t.me/+zptNJAJhDe41ZTEy) где обсуждаем архитектуру и баги.

Код должен соответствовать спецификации из `docs/specs/`. Любой PR нарушающий Integer Physics или SoA layout отклоняется без ревью — это не придирки, это физические законы движка.

---

## Лицензия

GPLv3 + коммерческое лицензирование. Открытый код — каждый может взять, запустить, встроить в своего робота.

---

## 💖 Поддержка проекта

Проект активно развивается. Вы можете поддержать его разработку! 

Ваши пожертвования пойдут на:
- Оплату Google AI Ultra & Claude Code Team/Enterprise и API, которые помогают быстрее писать код и приближают выход релиза.
- Оплату серверов и нужного железа для тестирования и разработки.

[☕ Поддержать на Ko-fi](https://ko-fi.com/axicor)

<p align="center">Copyright (C) 2026 Oleksandr Arzamazov</p>