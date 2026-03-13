#pragma once
#include <stdint.h>
#include <stddef.h>

#define MAX_DENDRITE_SLOTS 32
#define AXON_SENTINEL 0x80000000

struct alignas(32) BurstHeads8 {
    uint32_t h0; uint32_t h1; uint32_t h2; uint32_t h3;
    uint32_t h4; uint32_t h5; uint32_t h6; uint32_t h7;
};

// Строго 64 байта (1 кэш-линия)
struct alignas(64) VariantParameters {
    int32_t threshold;
    int32_t rest_potential;
    int32_t leak_rate;
    int32_t homeostasis_penalty;
    uint16_t homeostasis_decay;
    int16_t gsop_potentiation;
    int16_t gsop_depression;
    uint8_t refractory_period;
    uint8_t synapse_refractory_period;
    uint8_t slot_decay_ltm;
    uint8_t slot_decay_wm;
    uint8_t signal_propagation_length;
    uint8_t d1_affinity;
    uint16_t heartbeat_m;
    uint8_t d2_affinity;
    uint8_t ltm_slot_count;
    int16_t inertia_curve[15]; // !! Changed to 15 elements to match specification
    int16_t prune_threshold;   // 2 bytes
};

// Read-Only топология (Мапится из Flash-памяти)
struct FlashTopology {
    uint32_t* dendrite_targets;
    uint32_t* soma_to_axon; 
};

// Hot State (Лежит в 520 KB SRAM)
struct SramState {
    uint32_t padded_n;
    uint32_t total_axons;

    int32_t* voltage;
    uint8_t* flags;
    int32_t* threshold_offset;
    uint8_t* refractory_timer;

    int16_t* dendrite_weights;
    BurstHeads8* axon_heads;
};
