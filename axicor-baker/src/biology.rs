use serde::Deserialize;
use axicor_core::layout::VariantParameters;

#[derive(Debug, Deserialize)]
pub struct TomlNeuronType {
    pub is_inhibitory: bool,
    pub threshold: i32,
    pub rest_potential: i32,
    pub leak_shift: u32, // [DOD FIX]
    pub refractory_period: u8,
    pub spontaneous_firing_period_ticks: u32,
    pub homeostasis_penalty: i32,
    pub homeostasis_decay: u16,
    pub synapse_refractory_period: u8,
    pub signal_propagation_length: u8,
    pub initial_synapse_weight: u16,
    pub gsop_potentiation: u16,
    pub gsop_depression: u16,
    pub inertia_curve: [u8; 8], // [DOD FIX]
    pub ahp_amplitude: u16, // [DOD FIX]
    pub adaptive_leak_min_shift: i32, // [DOD FIX]
    pub adaptive_leak_gain: u16,
    pub adaptive_mode: u8,
    pub d1_affinity: u8,
    pub d2_affinity: u8,
}

impl From<TomlNeuronType> for VariantParameters {
    fn from(dto: TomlNeuronType) -> Self {
        Self {
            threshold: dto.threshold,
            rest_potential: dto.rest_potential,
            leak_shift: dto.leak_shift, // [DOD FIX]
            homeostasis_penalty: dto.homeostasis_penalty,
            spontaneous_firing_period_ticks: dto.spontaneous_firing_period_ticks,
            initial_synapse_weight: dto.initial_synapse_weight,
            gsop_potentiation: dto.gsop_potentiation,
            gsop_depression: dto.gsop_depression,
            homeostasis_decay: dto.homeostasis_decay,
            refractory_period: dto.refractory_period,
            synapse_refractory_period: dto.synapse_refractory_period,
            signal_propagation_length: dto.signal_propagation_length,
            is_inhibitory: dto.is_inhibitory as u8,
            inertia_curve: dto.inertia_curve, // [DOD FIX]
            ahp_amplitude: dto.ahp_amplitude, // [DOD FIX]
            _pad: [0; 6], // [DOD FIX] Keep C-ABI size rigid
            adaptive_leak_min_shift: dto.adaptive_leak_min_shift, // [DOD FIX]
            adaptive_leak_gain: dto.adaptive_leak_gain,
            adaptive_mode: dto.adaptive_mode,
            _leak_pad: [0; 3],
            d1_affinity: dto.d1_affinity,
            d2_affinity: dto.d2_affinity,
            heartbeat_m: if dto.spontaneous_firing_period_ticks > 0 {
                65536 / dto.spontaneous_firing_period_ticks
            } else {
                0
            },
        }
    }
}
