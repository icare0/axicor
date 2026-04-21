# GNM-Lib - Axicor Neuron Model Library

A library of digital neuron blueprints for the **Axicor** engine.
Each `.toml` file describes one biologically approximated neuron type - membrane properties, growth morphology, plasticity parameters - using Axicor integer physics.

---

## Table of Contents

1. [Directory Structure](#directory-structure)
2. [TOML File Format](#toml-file-format)
3. [Parameter Reference](#parameter-reference)
4. [Conversion Formulas](#conversion-formulas)
5. [Data Sources](#data-sources)
6. [Generation Pipeline](#generation-pipeline)
7. [Credits & Attribution](#credits--attribution)
8. [Invariants](#invariants)

---

## Directory Structure

```
GNM-Library/
|-- Cortex/                     # ~1780 types from Allen Cell Types API
|   |-- L1/                     # Cortical layer 1
|   |   |-- aspiny/             # Inhibitory interneurons
|   |   |-- sparsely spiny/     # Transitional type
|   |   \-- spiny/              # Excitatory pyramidal cells
|   |       |-- VISp5/          # Area -> nested variants
|   |       |   |-- 1.toml
|   |       |   \-- 2.toml
|   |       \-- SSp-un5.toml    # Single variant -> flat file
|   |-- L2/ ... L6b/            # Layers L2, L23, L3, L4, L5, L6, L6a, L6b
|
|-- Cerebellum/                 # Cerebellum (hardcoded from literature)
|   |-- Mouse/
|   |-- Rat/
|   \-- Zebrafish/
|
|-- Hippocampus/                # Hippocampus
|   |-- Mouse/
|   |-- Rat/
|
|-- Striatum/                   # Striatum
|   \-- Mouse/ Rat/
|
\-- Thalamus/                   # Thalamus
    \-- Mouse/ Rat/
```

### Naming Convention

**Cortex:** `L{layer}_{dendrite_type}_{brain_area}[_{index}]`
- Example: `L5_spiny_VISp5_42` - 42nd variant of a spiny neuron in area VISp5, layer L5

**Subcortical:** `{Region}_{CellType}`
- Example: `Thalamus_TC`, `Hippocampus_PV_Basket`

---

## TOML File Format

```toml
name = "L5_spiny_SSp-un5"
is_inhibitory = false

# Membrane
threshold = 35718                           # i32, uV (GNM)
rest_potential = 11607                      # i32, uV (GNM)
leak_rate = 204                             # i32, dV/tick
refractory_period = 9                       # u8, ticks
spontaneous_firing_period_ticks = 854       # u32, ticks (0 = off)

# Adaptation and Timings
homeostasis_penalty = 1000                  # i32, uV penalty per spike
homeostasis_decay = 2                       # u16, decay rate
synapse_refractory_period = 15              # u8, ticks
signal_propagation_length = 20              # u8, segments

# Growth and Morphology
steering_fov_deg = 34.4                     # f32, degrees
steering_radius_um = 66.7                   # f32, um
growth_vertical_bias = 0.95                 # f32, 0.0-1.0
dendrite_radius_um = 420.0                  # f32, um
type_affinity = 0.8                         # f32, 0.0-1.0
sprouting_weight_distance = 0.5             # f32, weight
sprouting_weight_power = 0.4                # f32, weight
sprouting_weight_explore = 0.1              # f32, weight
sprouting_weight_type = 0.2                 # f32, weight
steering_weight_inertia = 0.5               # f32, weight
steering_weight_sensor = 0.4                # f32, weight
steering_weight_jitter = 0.1                # f32, weight

# Synaptic Plasticity (GSOP)
initial_synapse_weight = 168                # u16, absolute
gsop_potentiation = 108                     # u16
gsop_depression = 69                        # u16
inertia_curve = [128, 121, 115, ...]        # [u8; 16], fixed-point (128 = 1.0x)
prune_threshold = 5                         # i16, min weight to survive
```

---

## Parameter Reference

### Membrane

| Parameter | Type | Units | Range | Description |
|-----------|------|-------|-------|-------------|
| `threshold` | i32 | uV (GNM) | 0-65,000 | Firing threshold. When potential >= threshold, the neuron generates a spike. |
| `rest_potential` | i32 | uV (GNM) | 0-30,000 | Resting potential. The membrane tends toward this value between spikes. |
| `leak_rate` | i32 | dV/tick | 10-12,000 | Membrane leak rate. Higher = faster return to rest_potential. |
| `refractory_period` | u8 | ticks | 5-25 | Absolute refractory period. The neuron cannot fire during this time. |
| `spontaneous_firing_period_ticks` | u32 | ticks | 0-100,000 | Spontaneous activity period (heartbeat). 0 = disabled. |

### Adaptation and Timings

| Parameter | Type | Units | Range | Description |
|-----------|------|-------|-------|-------------|
| `homeostasis_penalty` | i32 | uV | 1,000-20,000 | Threshold increase after each spike (spike-frequency adaptation). |
| `homeostasis_decay` | u16 | rate | 2-80 | Recovery rate of the adaptive threshold back to baseline. |
| `synapse_refractory_period` | u8 | ticks | 15 | Synapse refractory period between repeat activations. |
| `signal_propagation_length` | u8 | segments | 3-100 | Length of the Active Tail signal along the axon (1 segment ~ 20 um). |

### Growth and Morphology (Baker Pipeline)

| Parameter | Type | Units | Range | Description |
|-----------|------|-------|-------|-------------|
| `steering_fov_deg` | f32 | degrees | 20-180 | Cone Tracing field of view (axon target search). |
| `steering_radius_um` | f32 | um | 20-500 | Axon growth step radius. |
| `growth_vertical_bias` | f32 | - | 0.0-0.95 | Vertical growth bias (1.0 = strictly up/down through layers). |
| `dendrite_radius_um` | f32 | um | 15-500 | Dendritic field radius around the soma. |
| `type_affinity` | f32 | - | 0.0-1.0 | Attraction to somas of same vs different type. spiny=0.8, aspiny=0.2. |
| `sprouting_weight_distance` | f32 | - | 0.0-1.0 | Distance weight in Sprouting Score. |
| `sprouting_weight_power` | f32 | - | 0.0-1.0 | Signal power weight in Sprouting Score. |
| `sprouting_weight_explore` | f32 | - | 0.0-0.5 | Exploration weight in Sprouting Score. |
| `sprouting_weight_type` | f32 | - | 0.0-1.0 | Type matching weight in Sprouting Score. |
| `steering_weight_inertia` | f32 | - | 0.0-1.0 | Directional inertia weight during axon growth. |
| `steering_weight_sensor` | f32 | - | 0.0-1.0 | Sensory signal weight (attraction to targets). |
| `steering_weight_jitter` | f32 | - | 0.0-0.5 | Random noise weight during growth. |

### Synaptic Plasticity (GSOP - Axicor Synaptic Ordering Protocol)

| Parameter | Type | Units | Range | Description |
|-----------|------|-------|-------|-------------|
| `initial_synapse_weight` | u16 | - | 50-32,767 | Initial weight of a new synapse. ~1/50 of (threshold - rest). |
| `gsop_potentiation` | u16 | - | 2-32,767 | Potentiation rate (LTP). Higher = faster strengthening. |
| `gsop_depression` | u16 | - | 2-32,767 | Depression rate (LTD). Higher = faster weakening. |
| `inertia_curve` | [u8; 16] | fixed-point | 2-255 | Resistance to weight change based on rank. 128 = 1.0x (no modification). |
| `prune_threshold` | i16 | - | 5-8,000 | Minimum weight for synapse survival. Below this = pruning. |
| `is_inhibitory` | bool | - | - | true = inhibitory (GABA), false = excitatory (glutamate). |

---

## Conversion Formulas

### GNM Coordinate System

```
Biological Zero:     -82 mV = 0 uV (GNM)
Scale:               1 mV   = 1000 uV (GNM)
Time Tick:           0.1 ms = 1 tick
Ticks per second:    10,000
```

### Membrane Parameters

| Parameter | Formula | Allen Source |
|-----------|---------|--------------|
| `threshold` | `(ef__fast_trough_v_long_square - (-82)) * 1000` | `ef__fast_trough_v_long_square` |
| `rest_potential` | `(ef__vrest - (-82)) * 1000` | `ef__vrest` |
| `leak_rate` | `ef__tau * 10` | `ef__tau` (ms) |
| `refractory_period` | `29 - 4 * ef__upstroke_downstroke_ratio` , clamp [5, 25] | `ef__upstroke_downstroke_ratio_long_square` |
| `spontaneous_firing_period_ticks` | **Priority 1:** `10000 / ef__avg_firing_rate` | `ef__avg_firing_rate` (Hz) |
| | **Priority 2:** `10000 / ((0.5 * Rin) / dV)`, dV = threshold - rest | `ef__ri` (MOhm) |

### Adaptation

| Parameter | Formula | Allen Source |
|-----------|---------|--------------|
| `homeostasis_penalty` | `max(1000, ef__adaptation * 20000)` | `ef__adaptation` |
| `homeostasis_decay` | `250 / ef__avg_isi`, clamp [2, 80] | `ef__avg_isi` (ms) |
| `signal_propagation_length` | `nr__max_euclidean_distance / 20`, clamp [3, 100] | `nr__max_euclidean_distance` (um) |

### Growth Morphology

| Parameter | Formula | Note |
|-----------|---------|------|
| `steering_fov_deg` | aspiny: 150 deg, sparsely spiny: 75 deg, spiny: 40 deg - correction: `-min(20, axon_length/100)` | Interneurons = wide search, pyramids = narrow beam |
| `steering_radius_um` | `axon_total_length_um * 0.12`, clamp [20, 500] | 12% of axon length from SWC |
| `growth_vertical_bias` | **Exc:** `|soma_z - 0.05| * 2000 / axon_length`, clamp [0.3, 0.95] | L5 pyramids tend toward L1 |
| | **Inh:** `0.1 + soma_z * 0.2`, max 0.4 | Interneurons branch locally |
| `type_affinity` | spiny=0.8, aspiny=0.2, sparsely spiny=0.5 | Fixed rule |
| `sprouting_weight_explore` | `0.05 + nr__number_bifurcations / 300`, clamp [0.05, 0.5] | More bifurcations = more exploration |

### Steering weights (by axon length)

| Axon length | Type | distance | power | type | inertia | sensor | jitter |
|-------------|------|----------|-------|------|---------|--------|--------|
| < 300 um | Local interneuron | 0.7 | 0.3 | 0.3 | 0.2 | 0.6 | 0.2 |
| 300-800 um | Medium range | 0.5 | 0.4 | 0.2 | 0.5 | 0.4 | 0.1 |
| > 800 um | Long-range projector | 0.3 | 0.5 | 0.1 | 0.8 | 0.1 | 0.1 |

### GSOP Plasticity

| Parameter | Formula | Note |
|-----------|---------|------|
| `initial_synapse_weight` | 10 (Exc), 30 (Inh) | Safe Tabula Rasa |
| `gsop_potentiation` | **Exc:** `100 + adaptation * 400` | High adaptation = high plasticity |
| | **PV inh:** `200 + adaptation * 100` (ratio 100:1) | PVs maintain stable connections |
| `gsop_depression` | `potentiation * 1.2` | Entropy Erosion |
| `inertia_curve` | `128 * exp(-steepness * rank * 3.5 / 15)` | steepness = penalty_norm * 0.6 + adaptation * 0.4 |
| `prune_threshold` | `5` | Static Hard Limit |

### Dead Zone Guard (Patch)

Live learning condition for each rank:
```
(gsop_potentiation * inertia[rank]) >> 7 >= 1
```
If violated, `inertia[rank]` is raised to `ceil(128 / gsop_potentiation)`.

---

## Data Sources

### Cortex (~1780 neurons)

| Source | Provides | Endpoint / File |
|--------|----------|-----------------|
| **Allen Cell Types API** | Electrophysiology: Vrest, tau, Rin, firing rate, adaptation, upstroke/downstroke ratio, dendrite type, cortical layer, brain area | `https://api.brain-map.org/api/v2/data/query.json` model: `ApiCellTypesSpecimenDetail` |
| **Allen SWC morphometry** | Axon and dendrite morphology: `axon_total_length_um`, `dendrite_max_radius_um` | SWC files via `well_known_file_download/{nrwkf_id}` |

### Subcortical Structures (hardcoded)

Parameters defined manually based on literature:

| Region | Types | Sources |
|--------|-------|---------|
| **Thalamus** | TC, TRN | Sherman & Guillery 2006 |
| **Hippocampus** | CA1 Pyramidal, CA3 Pyramidal, Dentate Granule, PV Basket | Spruston 2008, Klausberger & Somogyi 2008 |
| **Striatum** | MSN | Kreitzer & Malenka 2008 |
| **Cerebellum** | Purkinje, Granule | Hausser & Clark 1997 |

---

## Generation Pipeline

The library was generated using an internal algorithm:

```
1. download_neuro_data.py      Downloads raw_data/ from Allen API + SWC
         |
2. generate_gnm_library.py     Allen JSON -> ~1780 individual .toml (Cortex)
   generate_subcortical_library.py  Hardcoded -> ~12 .toml (subcortical)
         |
3. patch_gnm_library.py        Post-hoc fixes:
                                 - signal_length >= refractory + 1
                                 - Dead Zone guard for inertia * potentiation
                                 - Initial_weight scaling based on delta
                                 - Prune_threshold synchronization
```

---

## Credits & Attribution

### Allen Institute for Brain Science

Cortical neuron electrophysiology and morphometry data derived from the
**Allen Cell Types Database**.

> (c) 2015 Allen Institute for Brain Science. Allen Cell Types Database.
> Available from: [celltypes.brain-map.org](https://celltypes.brain-map.org)
>
> Citation: *Allen Institute for Brain Science (2015). Allen Cell Types Database.
> Available from celltypes.brain-map.org*

The Allen Institute Terms of Use permit use of their data in research and derivative works
with attribution. GNM-Library `.toml` files are derivative works - biological parameters
(mV, ms, Hz, um) were transformed through custom conversion formulas into Axicor
integer physics units. No raw Allen data is redistributed.

### Literature Sources (Subcortical)

Subcortical neuron parameters are based on published electrophysiological measurements:

- Sherman, S. M. & Guillery, R. W. (2006). *Exploring the Thalamus and Its Role in Cortical Function.* MIT Press.
- Spruston, N. (2008). Pyramidal neurons: dendritic structure and synaptic integration. *Nature Reviews Neuroscience*, 9(3), 206-221.
- Klausberger, T. & Somogyi, P. (2008). Neuronal diversity and temporal dynamics. *Science*, 321(5885), 53-57.
- Kreitzer, A. C. & Malenka, R. C. (2008). Striatal plasticity and basal ganglia circuit function. *Neuron*, 60(4), 543-554.
- Hausser, M. & Clark, B. A. (1997). Tonic synaptic inhibition modulates neuronal output pattern. *Neuron*, 19(3), 665-678.

### License

GNM-Library is part of the Axicor project and is dual-licensed under MIT or Apache 2.0.
The conversion formulas and resulting neuron blueprints are original work; source data 
is used under the respective providers' terms of use.

---

## Invariants

- `inertia_curve` - exactly 16 elements `[u8; 16]`, 128 = 1.0x
- `is_inhibitory = true` -> synapse weight is interpreted as negative
- `signal_propagation_length >= refractory_period + 1` (required for signal propagation)
- `SUM sprouting_weight_* ~= 1.0`
- Dead Zone: `(gsop_potentiation * inertia[any_rank]) >> 7 >= 1`
