use axicor_core::config::blueprints::{BlueprintsConfig, AxicorConstantMemory, NeuronType};
use axicor_core::layout::VariantParameters;
use std::collections::HashMap;

/// Full node bootstrap sequence. Standard "Axicor Sequence" pipeline.
pub fn parse_blueprints(
    toml_content: &str,
) -> (AxicorConstantMemory, Vec<NeuronType>, HashMap<String, u8>) {
    let config =
        BlueprintsConfig::parse(toml_content).expect("Fatal: Failed to parse blueprints.toml");

    let num_types = config.neuron_types.len();
    if num_types > 16 {
        panic!(
            "Fatal: Architecture hard limit exceeded. Max 16 neuron types allowed, got {}. \
            (4-bit type_mask index constraint)",
            num_types
        );
    }

    let mut memory = AxicorConstantMemory {
        variants: [VariantParameters::default(); 16],
    };
    let mut name_map: HashMap<String, u8> = HashMap::new();

    for (i, nt) in config.neuron_types.iter().enumerate() {
        name_map.insert(nt.name.clone(), i as u8);

        memory.variants[i] = VariantParameters {
            threshold: nt.threshold,
            rest_potential: nt.rest_potential,
            leak_shift: nt.leak_shift,
            homeostasis_penalty: nt.homeostasis_penalty,
            spontaneous_firing_period_ticks: nt.spontaneous_firing_period_ticks,
            initial_synapse_weight: nt.initial_synapse_weight,
            gsop_potentiation: nt.gsop_potentiation,
            gsop_depression: nt.gsop_depression,
            homeostasis_decay: nt.homeostasis_decay,
            refractory_period: nt.refractory_period,
            synapse_refractory_period: nt.synapse_refractory_period,
            signal_propagation_length: nt.signal_propagation_length,
            is_inhibitory: nt.is_inhibitory as u8,
            inertia_curve: {
                let mut curve = [0u8; 8];
                for (idx, &val) in nt.inertia_curve.iter().enumerate().take(8) {
                    curve[idx] = val;
                }
                curve
            },
            ahp_amplitude: nt.ahp_amplitude,
            _pad: [0; 6],
            adaptive_leak_min_shift: nt.adaptive_leak_min_shift,
            adaptive_leak_gain: nt.adaptive_leak_gain,
            adaptive_mode: nt.adaptive_mode,
            _leak_pad: [0; 3],
            d1_affinity: nt.d1_affinity,
            d2_affinity: nt.d2_affinity,
            // [DOD FIX] Zero-cost: предвычисляем DDS-фазу при компиляции, чтобы избавить GPU от деления
            heartbeat_m: if nt.spontaneous_firing_period_ticks > 0 {
                65536 / nt.spontaneous_firing_period_ticks
            } else {
                0
            },
        };
    }

    (memory, config.neuron_types, name_map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_blueprints_happy_path() {
        let toml = r#"
            [[neuron_type]]
            name = "Excitatory"
            threshold = 1000
            rest_potential = 500
            leak_shift = 2
            refractory_period = 5
            synapse_refractory_period = 5
            homeostasis_penalty = 100
            homeostasis_decay = 5

            [[neuron_type]]
            name = "Inhibitory"
            threshold = 800
            rest_potential = 400
            leak_shift = 3
            refractory_period = 3
            synapse_refractory_period = 3
            homeostasis_penalty = 80
            homeostasis_decay = 3
            is_inhibitory = true
        "#;

        let (memory, types, name_map) = parse_blueprints(toml);

        assert_eq!(types.len(), 2);
        assert_eq!(name_map.get("Excitatory"), Some(&0));
        assert_eq!(name_map.get("Inhibitory"), Some(&1));

        assert_eq!(memory.variants[0].threshold, 1000);
        assert_eq!(memory.variants[0].is_inhibitory, 0);
        assert_eq!(memory.variants[1].threshold, 800);
        assert_eq!(memory.variants[1].is_inhibitory, 1);
    }

    #[test]
    fn test_parse_blueprints_limit() {
        let mut toml = String::new();
        for i in 0..16 {
            toml.push_str(&format!(
                r#"
                [[neuron_type]]
                name = "Type{}"
                threshold = 1000
                rest_potential = 500
                leak_shift = 1
                refractory_period = 5
                synapse_refractory_period = 5
                homeostasis_penalty = 100
                homeostasis_decay = 5
                "#,
                i
            ));
        }

        let (memory, types, name_map) = parse_blueprints(&toml);
        assert_eq!(types.len(), 16);
        assert_eq!(name_map.len(), 16);
    }

    #[test]
    #[should_panic(expected = "Fatal: Architecture hard limit exceeded")]
    fn test_parse_blueprints_exceed_limit() {
        let mut toml = String::new();
        for i in 0..17 {
            toml.push_str(&format!(
                r#"
                [[neuron_type]]
                name = "Type{}"
                threshold = 1000
                rest_potential = 500
                leak_shift = 1
                refractory_period = 5
                synapse_refractory_period = 5
                homeostasis_penalty = 100
                homeostasis_decay = 5
                "#,
                i
            ));
        }
        parse_blueprints(&toml);
    }

    #[test]
    #[should_panic(expected = "Fatal: Failed to parse blueprints.toml")]
    fn test_parse_blueprints_invalid_toml() {
        let toml = "invalid = toml";
        parse_blueprints(toml);
    }
}
