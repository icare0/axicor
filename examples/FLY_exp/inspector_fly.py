# Вспомагательный скрипт для инспекции flygym
#!/usr/bin/env python3
import numpy as np
from flygym.mujoco import NeuroMechFly

def inspect():
    print("🪰 Инициализация NeuroMechFly для инспекции...")
    env = NeuroMechFly()
    obs_space = env.observation_space
    act_space = env.action_space

    print("\n" + "="*50)
    print("ОБСЕРВАЦИИ (ВХОДЫ В СЕТЧАТКУ И СЕНСОРЫ)")
    print("="*50)
    total_obs = 0
    for k, v in obs_space.items():
        size = int(np.prod(v.shape))
        total_obs += size
        print(f"[{k}] Size: {size}, Shape: {v.shape}")
        # Выводим реальные лимиты, чтобы избежать деления на 0 при нормализации
        _min = v.low.flatten()
        _max = v.high.flatten()
        print(f"    Min limits: {_min[:4]} ... {_min[-4:]}")
        print(f"    Max limits: {_max[:4]} ... {_max[-4:]}")
        
        # Поиск потенциальных мертвых зон (где min == max)
        dead_zones = np.where(_min == _max)
        if len(dead_zones) > 0:
            print(f"    ⚠️ DEAD ZONES FOUND at indices: {dead_zones}")

    print(f"\n[ИТОГО СЕНСОРОВ]: {total_obs} float-переменных")

    print("\n" + "="*50)
    print("АКТУАТОРЫ (ВЫХОДЫ МОТОРНОЙ КОРЫ)")
    print("="*50)
    for k, v in act_space.items():
        size = int(np.prod(v.shape))
        print(f"[{k}] Size: {size}, Shape: {v.shape}")
        _min = v.low.flatten()
        _max = v.high.flatten()
        print(f"    Min limits: {_min[:4]} ...")
        print(f"    Max limits: {_max[:4]} ...")

if __name__ == "__main__":
    inspect()
