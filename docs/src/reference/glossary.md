# Glossary

> Part of the Axicor architecture. Complete terminology reference.

## A

**Active Tail**
The trailing edge of a dendritic tree segment that contains soma-proximal synapses still receiving spike signals. Contrasts with the "dormant head" in pruned branches.

**Anatomical Blueprint (Anatomy)**
TOML-formatted specification file (`anatomy.toml`) defining brain zone topology, layer arrangement (Z-order sorting), and density.

**Axon Sentinel**
A boundary marker (`0x80000000`) at the end of each neuron's axon output array on the GPU. Prevents integer overflow and enables O(1) detection of inactive axons during spike propagation.

**Axicor-Baker**
The offline CPU compiler utility. Converts TOML configurations into flattened, warp-aligned C-ABI binary dumps (`.state`, `.axons`, `.paths`, `.ghosts`).

## B

**Baking (Compilation Pipeline)**
A multi-stage process executed by `axicor-baker` converting human-readable TOML files into GPU-ready Headerless SoA binary formats via Cone Tracing and spatial hashing.

**Brain Shard (Shard)**
An independently-executable GPU subdivision of a zone. Has a dedicated VRAM allocation and runs Day Phase kernels autonomously.

**BSP Barrier (Bulk Synchronous Parallel)**
An inter-shard synchronization primitive ensuring all nodes complete their autonomous Day Phase batches before exchanging spikes over the network.

## C

**Charge Domain**
The electrical representation of synaptic weights (in microvolts) used during the Day Phase GLIF calculations. Derived from the Mass Domain via a 16-bit right bit-shift (`>> 16`).

## D

**Day Phase**
The hot loop executed exclusively on the GPU/MCU. Performs branchless Integer Physics calculations (GLIF, GSOP, Axon Propagation) without any structural topology changes or dynamic allocations.

**Dense Index**
A continuous integer index (`0..N-1`) used by the GPU in the Day Phase to access SoA arrays. The total size `N` is padded to a multiple of 64 to guarantee Warp Alignment.

## G

**Ghost Axon**
A virtual representation of an axon that belongs to a neuron in another shard/zone, but has dendrites connected to it in the local shard. Updated via network UDP packets.

## I

**Integer Physics**
An architectural law of the Axicor engine. All membrane, plasticity, and routing calculations are performed using 100% branchless integer math (no FPU instructions), guaranteeing absolute cross-platform determinism.

## M

**Mass Domain**
The structural representation of synaptic weights used for STDP learning (ranging up to 2.14 billion). It prevents micro-experiences from altering the electrical Charge Domain immediately, acting as a buffer for memory consolidation.

## N

**Night Phase**
The offline maintenance phase executed on the CPU. Performs structural plasticity: column defragmentation, synaptic pruning, and axon sprouting.

## P

**Packed Position**
A 32-bit integer encoding a neuron's spatial coordinates and type (`[Type:4 | Z:8 | Y:10 | X:10]`). Used exclusively during Baking and the Night Phase for Cone Tracing.

## Z

**Zero-Copy DMA**
The methodology of loading `.state` and `.axons` files directly into VRAM (or mapping to OS memory via `mmap`) exactly as they reside on disk, without any parsing, serialization, or object allocation.