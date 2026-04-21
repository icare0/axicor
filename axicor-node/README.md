# axicor-node

The distributed simulation runtime for Axicor, acting as the orchestrator for local shards and the gateway for external I/O.

## Technical Focus

- **BSP Orchestrator:** Implements Bulk Synchronous Parallel (BSP) logic to keep multiple shards and nodes in perfect lockstep.
- **Lock-Free UDP Fast-Path:** A high-frequency I/O layer designed for sub-millisecond sensory input and motor output, bypassing traditional kernel bottlenecks.
- **Day/Night Asynchronous Phases:** Manages the strict separation between the simulation Hot Loop (Day) and the background structural plasticity tasks (Night).
- **RCU Routing:** Uses Read-Copy-Update (RCU) primitives to handle dynamic network topology changes without blocking the spike propagation path.

## Components

- **Node Runtime:** The main simulation loop.
- **I/O Server:** Manages UDP GSIO/GSOO packets.
- **Orchestrator:** Coordinates the GPU kernels and CPU baker daemons.
- **Telemetry Server:** Provides real-time spikes and voltage data via WebSockets.

## License
Dual-licensed under MIT or Apache 2.0.
