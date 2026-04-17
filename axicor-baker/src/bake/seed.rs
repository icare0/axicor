/// Re-export of seed primitives from axicor-core.
/// Baker uses this module directly, while runtime and other crates fetch from axicor-core.

pub use axicor_core::seed::{
    entity_seed,
    random_f32,
    seed_from_str,
    shuffle_indices,
};
