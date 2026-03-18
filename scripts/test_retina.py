#!/usr/bin/env python3
import time
import gc
import numpy as np
import cv2
import sys
import os

# Добавляем путь к SDK
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "genesis-client")))
from genesis.retina import RetinaEncoder

def run_retina_stress_test():
    W, H = 256, 256
    B = 20  # sync_batch_ticks
    
    print(f"👁️ Инициализация RetinaEncoder ({W}x{H}, Batch={B} ticks)...")
    retina = RetinaEncoder(width=W, height=H, batch_size=B)

    # 1. C-ABI Warp Alignment Check
    expected_bytes_per_tick = ((W * H + 31) // 32) * 4
    expected_total = expected_bytes_per_tick * B
    
    print(f"📊 C-ABI Контракт:")
    print(f"   - Пикселей: {W * H}")
    print(f"   - Байт на тик: {expected_bytes_per_tick} (Выровнено по 32 бита: {expected_bytes_per_tick % 4 == 0})")
    print(f"   - Payload size: {expected_total} байт")
    
    assert retina.total_bytes == expected_total, f"FATAL: C-ABI Alignment broken! Expected {expected_total}, got {retina.total_bytes}"

    # 2. Имитация сетевого буфера (с учетом 20 байт ExternalIoHeader)
    tx_buffer = bytearray(20 + expected_total)
    tx_view = memoryview(tx_buffer)

    # 3. Преаллокация фейкового кадра (Zero-Garbage)
    mock_frame = np.zeros((H, W, 3), dtype=np.uint8)

    # Изолируем сборщик мусора
    gc.collect()
    gc.disable()
    # Фиксируем начальный счетчик (объекты в Gen 0)
    start_gen0 = gc.get_count()[0]

    ITERATIONS = 10_000
    print(f"\n🔥 Запуск Hot Loop на {ITERATIONS} кадров...")
    
    start_t = time.perf_counter()
    for _ in range(ITERATIONS):
        # In-place мутация кадра (имитация шума матрицы камеры) 
        # Не выделяет ни одного байта в куче Питона
        cv2.randu(mock_frame, 0, 255)
        
        # Кодирование
        retina.encode_into(mock_frame, tx_view, offset=20)

    elapsed = time.perf_counter() - start_t
    
    end_gen0 = gc.get_count()[0]
    gc.enable()

    fps = ITERATIONS / elapsed
    tps = fps * B
    # Полоса пропускания в Мбит/с
    bandwidth_mbps = (expected_total * 8 * fps) / (1024**2)

    print("\n" + "="*40)
    print("🏁 РЕЗУЛЬТАТЫ ПРОФИЛИРОВАНИЯ")
    print("="*40)
    print(f"⏱ Время выполнения:   {elapsed:.3f} сек")
    print(f"🎥 Производительность: {fps:,.0f} FPS")
    print(f"⚡ Сетевой эквивалент: {tps:,.0f} TPS (Тиков в секунду)")
    print(f"📡 Поток данных:      {bandwidth_mbps:.2f} Mbps")
    print(f"🗑 Объектов в Gen 0:   {start_gen0} -> {end_gen0} (Дельта: {end_gen0 - start_gen0})")
    
    if end_gen0 - start_gen0 > 10:
        print(f"❌ ПРОВАЛ: Обнаружена скрытая аллокация в Hot Loop! (Дельта: {end_gen0 - start_gen0})")
        sys.exit(1)
    else:
        print("✅ ZERO-GARBAGE ИНВАРИАНТ ПОДТВЕРЖДЕН")

if __name__ == '__main__':
    run_retina_stress_test()
