import socket
import struct
import random
import gymnasium as gym
import numpy as np
import threading
import time

# ═══════════════════════════════════════════════════════════════
# Genesis AntV4 Client — Нейроморфный контроллер муравья
# ═══════════════════════════════════════════════════════════════

GSIO_MAGIC = 0x4F495347  # "GSIO"
GSOO_MAGIC = 0x4F4F5347  # "GSOO"

def fnv1a_32(data: bytes) -> int:
    h = 0x811c9dc5
    for b in data:
        h ^= b
        h = (h * 0x01000193) & 0xFFFFFFFF
    return h

# ─── Конфигурация ──────────────────────────────────────────────
GENESIS_IP    = "127.0.0.1"
PORT_OUT      = 8081   # Порт отправки (к ноде)
PORT_IN       = 8082   # Порт приёма (от ноды)
BATCH_TICKS   = 100    # Размер батча (должен совпадать с sync_batch_ticks)
WORDS_PER_TICK = 11    # 324 виртуальных аксона / 32 = 10.125 → 11 слов

# Хэши зон и матриц (FNV1a)
ZONE_HASH        = fnv1a_32(b"SensoryCortex")
MATRIX_PROP_HASH = fnv1a_32(b"proprioception_joints")
MOTOR_ZONE_HASH  = fnv1a_32(b"MotorCortex")
MOTOR_MATRIX_HASH = fnv1a_32(b"motor_actuators")

# ─── Условия ресета ────────────────────────────────────────────
STUCK_LIMIT_WARMUP  = 5000    # Фаза прогрева: много времени на случайные пробы
STUCK_LIMIT_NORMAL  = 2000    # После прогрева: умеренная терпимость
WARMUP_EPISODES     = 50      # Сколько эпизодов считается «прогревом»
FLIP_THRESHOLD      = 0.15    # Z < 0.15 = реально перевернулся (не просто наклон)
PROGRESS_DELTA      = 0.003   # Минимальное продвижение по X за шаг

# ─── Допамин (Reward Shaping) ──────────────────────────────────
DOPAMINE_FORWARD_SCALE = 8000.0   # Масштаб за движение вперёд
DOPAMINE_ALIVE_BONUS   = 50       # Бонус за каждый батч пока жив
DOPAMINE_HEIGHT_SCALE  = 2000.0   # Бонус за вертикальное положение (не лежит)


class State:
    def __init__(self):
        self.obs = np.zeros(27)
        self.action = np.zeros(8)
        self.running = True
        self.steps_no_progress = 0
        self.last_x = 0.0
        self.episode = 0
        self.episode_steps = 0
        self.total_steps = 0
        self.best_distance = 0.0
        self.episode_start_x = 0.0

state = State()


def encode_population(value, min_val, max_val, neurons=16):
    """Кодирует скалярное значение в population code (битовая маска)."""
    norm = np.clip((value - min_val) / (max_val - min_val), 0.0, 1.0)
    center_idx = int(norm * (neurons - 1))
    bitmask = 0
    for i in range(max(0, center_idx - 1), min(neurons, center_idx + 2)):
        bitmask |= (1 << i)
    return bitmask


def compute_dopamine(obs, last_x):
    """
    Многокомпонентный допаминовый сигнал:
    1. Forward: движение по X (основной)
    2. Alive: бонус за пребывание в вертикальном положении
    3. Height: штраф за низкий центр масс (лежит/падает)
    """
    current_x = obs[0]
    current_z = obs[2] if len(obs) > 2 else 0.5

    # 1. Forward movement (главный сигнал)
    dx = current_x - last_x
    forward = dx * DOPAMINE_FORWARD_SCALE

    # 2. Alive bonus (подкрепление за выживание)
    alive = DOPAMINE_ALIVE_BONUS

    # 3. Height bonus (чем выше центр масс — тем лучше, побуждает стоять)
    height_bonus = (current_z - 0.3) * DOPAMINE_HEIGHT_SCALE

    total = forward + alive + height_bonus
    return int(np.clip(total, -2048, 2048))


def get_stuck_limit():
    """Лимит терпимости зависит от фазы обучения."""
    if state.episode < WARMUP_EPISODES:
        return STUCK_LIMIT_WARMUP
    return STUCK_LIMIT_NORMAL


def udp_hot_loop():
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.bind(("0.0.0.0", PORT_IN))
    sock.settimeout(0.001)
    print(f"💉 [Client] UDP Loop Started (Batch: {BATCH_TICKS} ticks)")

    while state.running:
        # [DOD] Идеально выровненная матрица [Тики x Слова]
        batch_bitmask = np.zeros((BATCH_TICKS, WORDS_PER_TICK), dtype=np.uint32)
        total_dopamine = 0

        for t in range(BATCH_TICKS):
            obs = state.obs

            # 1. Proprioception: 8 суставов × 2 значения = 16 → Слова 0..7
            for i in range(0, 16, 2):
                m1 = encode_population(obs[i+11], -1.2, 1.2, neurons=16)
                m2 = encode_population(obs[i+12], -1.2, 1.2, neurons=16)
                batch_bitmask[t, i//2] = m1 | (m2 << 16)

            # 2. Vestibular: гироскоп + ускорения → Слова 8..9
            for i in range(0, 8, 4):
                v0 = encode_population(obs[i+3], -2.5, 2.5, neurons=8)
                v1 = encode_population(obs[i+4], -2.5, 2.5, neurons=8)
                v2 = encode_population(obs[i+5], -2.5, 2.5, neurons=8)
                v3 = encode_population(obs[i+6], -2.5, 2.5, neurons=8)
                batch_bitmask[t, 8 + i//4] = v0 | (v1 << 8) | (v2 << 16) | (v3 << 24)

            # 3. Tactile: контакт лап с землёй → Слово 10
            tact_mask = 0
            for i in range(4):
                if obs[i] > 0.0:
                    tact_mask |= (1 << i)
            batch_bitmask[t, 10] = tact_mask

            total_dopamine += compute_dopamine(obs, state.last_x)
            state.last_x = obs[0]

            time.sleep(0.0001)

        avg_dopamine = total_dopamine // BATCH_TICKS

        # Отправляем монолитный блоб
        payload = batch_bitmask.tobytes()
        packet = struct.pack(f"<IIIIhH",
            GSIO_MAGIC, ZONE_HASH, MATRIX_PROP_HASH,
            len(payload), avg_dopamine, 0
        ) + payload
        sock.sendto(packet, (GENESIS_IP, PORT_OUT))

        # Читаем моторные выходы
        try:
            data, _ = sock.recvfrom(65535)
            if len(data) >= 20:
                magic, z_hash, m_hash, p_size, reward, _ = struct.unpack("<IIIIhH", data[:20])

                if magic == GSOO_MAGIC and z_hash == MOTOR_ZONE_HASH:
                    payload = data[20:20+p_size]

                    # [DOD] Декодируем батч. Time Integration через суммирование.
                    spikes = np.frombuffer(payload, dtype=np.uint8).reshape((BATCH_TICKS, 256))
                    total_spikes = np.sum(spikes, axis=0, dtype=np.int32)

                    for i in range(8):
                        flexor  = np.sum(total_spikes[(i*2)*16     : (i*2+1)*16])
                        extensor = np.sum(total_spikes[(i*2+1)*16 : (i*2+2)*16])
                        # Population Rate Code
                        state.action[i] = ((flexor - extensor) / (BATCH_TICKS * 16.0)) * 2.0

        except socket.timeout:
            pass


def log_episode_end(reason):
    """Логирует статистику по завершённому эпизоду."""
    dist = state.obs[0] - state.episode_start_x
    prefix = "🔥" if dist > state.best_distance else "📊"
    if dist > state.best_distance:
        state.best_distance = dist

    phase = "WARMUP" if state.episode < WARMUP_EPISODES else "LEARN"
    print(f"{prefix} [Ep {state.episode:4d}] {phase} | {reason:10s} | "
          f"steps={state.episode_steps:5d} | dist={dist:+.3f} | "
          f"best={state.best_distance:.3f} | total={state.total_steps}")


def main():
    try:
        env = gym.make('Ant-v4', render_mode="human",
                       exclude_current_positions_from_observation=False)
        print("📺 [Client] Rendering: HUMAN")
    except Exception:
        env = gym.make('Ant-v4', render_mode="rgb_array",
                       exclude_current_positions_from_observation=False)
        print("📺 [Client] Rendering: RGB_ARRAY (no display)")

    state.obs, _ = env.reset()
    state.last_x = state.obs[0]
    state.episode_start_x = state.obs[0]

    udp_thread = threading.Thread(target=udp_hot_loop, daemon=True)
    udp_thread.start()

    print(f"🧠 [Client] Warmup: {WARMUP_EPISODES} episodes (stuck_limit={STUCK_LIMIT_WARMUP})")
    print(f"🧠 [Client] Normal: stuck_limit={STUCK_LIMIT_NORMAL}, flip={FLIP_THRESHOLD}")

    try:
        while True:
            state.obs, reward, terminated, truncated, info = env.step(state.action)
            state.episode_steps += 1
            state.total_steps += 1

            # Прогресс по X?
            if state.obs[0] > state.last_x + PROGRESS_DELTA:
                state.steps_no_progress = 0
            else:
                state.steps_no_progress += 1

            # ─── Условия ресета ────────────────────────────
            flipped = state.obs[2] < FLIP_THRESHOLD
            stuck   = state.steps_no_progress > get_stuck_limit()

            reason = None
            if terminated:  reason = "TERMINATED"
            elif truncated: reason = "TRUNCATED"
            elif flipped:   reason = "FLIPPED"
            elif stuck:     reason = "STUCK"

            if reason:
                log_episode_end(reason)
                state.obs, _ = env.reset()
                state.episode += 1
                state.episode_steps = 0
                state.steps_no_progress = 0
                state.last_x = state.obs[0]
                state.episode_start_x = state.obs[0]

    except KeyboardInterrupt:
        print(f"\n🛑 [Client] Shutdown. Total episodes: {state.episode}, "
              f"Total steps: {state.total_steps}, Best distance: {state.best_distance:.3f}")
        state.running = False
        udp_thread.join(timeout=2)
        env.close()


if __name__ == "__main__":
    main()
