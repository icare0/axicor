#include <stdio.h>
#include <stdlib.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "esp_timer.h"
#include <inttypes.h>
#include "genesis_core.hpp"

VariantParameters VARIANT_LUT[2];
SramState sram;
FlashTopology flash;
int16_t global_dopamine = 0; // Для R-STDP

void init_brain(uint32_t num_neurons) {
    sram.padded_n = num_neurons;
    sram.total_axons = num_neurons;

    // SRAM Allocations (Hot Data)
    sram.voltage = (int32_t*)calloc(num_neurons, sizeof(int32_t));
    sram.flags = (uint8_t*)calloc(num_neurons, sizeof(uint8_t));
    sram.threshold_offset = (int32_t*)calloc(num_neurons, sizeof(int32_t));
    sram.refractory_timer = (uint8_t*)calloc(num_neurons, sizeof(uint8_t));
    sram.dendrite_weights = (int16_t*)calloc(num_neurons * MAX_DENDRITE_SLOTS, sizeof(int16_t));
    sram.axon_heads = (BurstHeads8*)calloc(num_neurons, sizeof(BurstHeads8));

    // Flash Allocations (В реальном чипе это будет mmap_ptr)
    flash.dendrite_targets = (uint32_t*)calloc(num_neurons * MAX_DENDRITE_SLOTS, sizeof(uint32_t));
    flash.soma_to_axon = (uint32_t*)calloc(num_neurons, sizeof(uint32_t));

    for(uint32_t i = 0; i < num_neurons; i++) {
        sram.axon_heads[i].h0 = AXON_SENTINEL;
    }

    // Базовый профиль с GSOP
    VARIANT_LUT[0].threshold = 400;
    VARIANT_LUT[0].rest_potential = 0;
    VARIANT_LUT[0].leak_rate = 10;
    VARIANT_LUT[0].refractory_period = 5;
    VARIANT_LUT[0].signal_propagation_length = 3;
    VARIANT_LUT[0].gsop_potentiation = 60;
    VARIANT_LUT[0].gsop_depression = 30;
    VARIANT_LUT[0].ltm_slot_count = 16;
    VARIANT_LUT[0].slot_decay_ltm = 128; // 1.0x
    VARIANT_LUT[0].slot_decay_wm = 64;   // 0.5x
    VARIANT_LUT[0].d1_affinity = 128;
    VARIANT_LUT[0].d2_affinity = 128;
    for(int i=0; i<15; i++) VARIANT_LUT[0].inertia_curve[i] = 128 - (i * 8);

    printf("🧠 Genesis-Lite: %" PRIu32 " neurons. Memory Split: SRAM / Flash.\n", num_neurons);
}

// Branchless min distance для BurstHeads8
inline uint32_t check_head_dist(uint32_t head, uint32_t seg_idx, uint32_t prop_len, uint32_t current_min) {
    uint32_t d = head - seg_idx;
    return (d <= prop_len && d < current_min) ? d : current_min;
}

void day_phase_task(void *pvParameter) {
    uint32_t tick = 0;
    uint32_t v_seg = 1; 

    while(1) {
        int64_t start_time = esp_timer_get_time();

        // 1. Propagate Axons
        for(uint32_t i = 0; i < sram.total_axons; i++) {
            if(sram.axon_heads[i].h0 != AXON_SENTINEL) sram.axon_heads[i].h0 += v_seg;
            if(sram.axon_heads[i].h1 != AXON_SENTINEL) sram.axon_heads[i].h1 += v_seg;
        }

        // 2. Update Neurons (GLIF + Dendrites)
        for(uint32_t tid = 0; tid < sram.padded_n; tid++) {
            uint8_t flags = sram.flags[tid];
            uint8_t variant_id = (flags >> 4) & 0x0F;
            VariantParameters p = VARIANT_LUT[variant_id]; 
            
            if (sram.refractory_timer[tid] > 0) {
                sram.refractory_timer[tid] -= 1;
                sram.flags[tid] &= ~0x01; 
                continue;
            }

            int32_t current_voltage = sram.voltage[tid];
            int32_t i_in = 0;

            for (int slot = 0; slot < MAX_DENDRITE_SLOTS; slot++) {
                uint32_t col_idx = slot * sram.padded_n + tid;
                uint32_t target_packed = flash.dendrite_targets[col_idx];
                
                if (target_packed == 0) break; 

                uint32_t seg_idx = target_packed >> 24;
                uint32_t axon_id = (target_packed & 0x00FFFFFF) - 1;

                BurstHeads8 h = sram.axon_heads[axon_id];
                uint32_t prop = p.signal_propagation_length;

                bool hit = ((h.h0 - seg_idx) <= prop) || ((h.h1 - seg_idx) <= prop); // Упрощено для скорости
                if (hit) {
                    i_in += sram.dendrite_weights[col_idx];
                }
            }

            current_voltage += i_in;
            int32_t diff = current_voltage - p.rest_potential;
            int32_t sign = (diff > 0) - (diff < 0);
            int32_t leaked_abs = abs(diff) - p.leak_rate;
            leaked_abs = leaked_abs & ~(leaked_abs >> 31); 
            current_voltage = p.rest_potential + (sign * leaked_abs);

            int32_t effective_threshold = p.threshold + sram.threshold_offset[tid];
            if (current_voltage >= effective_threshold) {
                current_voltage = p.rest_potential;
                sram.refractory_timer[tid] = p.refractory_period;
                sram.flags[tid] = (flags & 0xFE) | 0x01; // Устанавливаем бит спайка
                
                // Сдвиг пулеметной очереди
                sram.axon_heads[tid].h1 = sram.axon_heads[tid].h0;
                sram.axon_heads[tid].h0 = 0; 
            } else {
                sram.flags[tid] &= ~0x01;
            }

            sram.voltage[tid] = current_voltage;
        }

        // 3. Apply GSOP (R-STDP Plasticity)
        for(uint32_t tid = 0; tid < sram.padded_n; tid++) {
            uint8_t flags = sram.flags[tid];
            if ((flags & 0x01) == 0) continue; // Пластичность только при спайке сомы

            uint8_t variant_id = (flags >> 4) & 0x0F;
            VariantParameters p = VARIANT_LUT[variant_id]; 
            int32_t dopamine = global_dopamine;

            for (int slot = 0; slot < MAX_DENDRITE_SLOTS; slot++) {
                uint32_t col_idx = slot * sram.padded_n + tid;
                uint32_t target_packed = flash.dendrite_targets[col_idx];
                if (target_packed == 0) break;

                uint32_t seg_idx = target_packed >> 24;
                uint32_t axon_id = (target_packed & 0x00FFFFFF) - 1;
                BurstHeads8 h = sram.axon_heads[axon_id];
                uint32_t prop = p.signal_propagation_length;

                uint32_t min_dist = 0xFFFFFFFF;
                min_dist = check_head_dist(h.h0, seg_idx, prop, min_dist);
                min_dist = check_head_dist(h.h1, seg_idx, prop, min_dist);

                bool is_active = (min_dist != 0xFFFFFFFF);
                int16_t w = sram.dendrite_weights[col_idx];
                int32_t w_sign = (w >= 0) ? 1 : -1;
                int32_t abs_w = w >= 0 ? w : -w;

                uint32_t rank = abs_w >> 11;
                if (rank > 14) rank = 14;
                int32_t inertia = p.inertia_curve[rank];

                int32_t pot_mod = (dopamine * p.d1_affinity) >> 7;
                int32_t dep_mod = (dopamine * p.d2_affinity) >> 7;

                int32_t raw_pot = p.gsop_potentiation + pot_mod;
                int32_t raw_dep = p.gsop_depression - dep_mod;
                int32_t final_pot = raw_pot & ~(raw_pot >> 31);
                int32_t final_dep = raw_dep & ~(raw_dep >> 31);

                int32_t delta_pot = (final_pot * inertia) >> 7;
                int32_t delta_dep = (final_dep * inertia) >> 7;

                uint32_t cooling_shift = is_active ? (min_dist >> 4) : 0;
                int32_t delta = is_active ? (delta_pot >> cooling_shift) : -delta_dep;

                int32_t decay = (slot >= p.ltm_slot_count) ? p.slot_decay_wm : p.slot_decay_ltm;
                delta = (delta * decay) >> 7;

                int32_t new_abs = abs_w + delta;
                if (new_abs < 1) new_abs = 1;
                if (new_abs > 32767) new_abs = 32767;

                sram.dendrite_weights[col_idx] = (int16_t)(new_abs * w_sign);
            }
        }

        int64_t end_time = esp_timer_get_time();
        tick++;
        if (tick % 100 == 0) {
            printf("⚡ Tick %" PRIu32 " | Hot loop time: %" PRId64 " us\n", tick, (int64_t)(end_time - start_time));
        }

        if (tick % 10 == 0) {
            vTaskDelay(1 / portTICK_PERIOD_MS); // Watchdog yield
        }
    }
}

extern "C" void app_main(void) {
    printf("🚀 Booting Genesis-Lite on FreeRTOS (Dual-Core)...\n");
    init_brain(256);

    xTaskCreatePinnedToCore(
        day_phase_task, "DayPhase", 8192, NULL, configMAX_PRIORITIES - 1, NULL, 1 
    );
}
