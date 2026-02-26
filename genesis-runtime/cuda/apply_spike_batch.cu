#include <cuda_runtime.h>
#include <stdint.h>

__global__ void apply_spike_batch_kernel(uint32_t num_spikes,
                                         const uint32_t *__restrict__ schedule_indices,
                                         uint32_t *axon_heads,
                                         uint32_t total_axons) {
  uint32_t tid = blockIdx.x * blockDim.x + threadIdx.x;

  if (tid < num_spikes) {
    // Direct receiver Ghost ID from the O(1) sender-mapped array
    uint32_t ghost_id = schedule_indices[tid];

    // Bounds check to protect against stale cross-shard messages (freed ghosts)
    if (ghost_id < total_axons) {
      // Zero out the axon head to simulate a fresh spike arriving
      axon_heads[ghost_id] = 0;
    }
  }
}

// Redefine the ffi function implementation properly
extern "C" void launch_apply_spike_batch_impl(uint32_t num_spikes,
                                              const uint32_t *schedule_indices,
                                              uint32_t *axon_heads,
                                              uint32_t total_axons,
                                              void *stream) {
  if (num_spikes == 0)
    return;
  int blockSize = 128;
  int numBlocks = (num_spikes + blockSize - 1) / blockSize;
  apply_spike_batch_kernel<<<numBlocks, blockSize, 0, (cudaStream_t)stream>>>(
      num_spikes, schedule_indices, axon_heads, total_axons);
}
