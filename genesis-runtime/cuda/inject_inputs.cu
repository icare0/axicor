#include <cuda_runtime.h>
#include <stdint.h>

// 05_signal_physics.md §2.4 Kernel (InjectInputs)
__global__ void inject_inputs_kernel(uint32_t *axon_heads,
                                     const uint32_t *input_bitmask,
                                     const uint32_t *map_pixel_to_axon,
                                     uint32_t num_pixels,
                                     uint32_t tick_in_batch) {
  uint32_t tid = blockIdx.x * blockDim.x + threadIdx.x;
  if (tid >= num_pixels)
    return;

  uint32_t words_per_tick = (num_pixels + 31) / 32;
  uint32_t offset = tick_in_batch * words_per_tick;

  // Broadcast read: 32 потока варпа → 1 u32 из bitmask
  uint32_t mask = input_bitmask[offset + (tid / 32)];
  uint32_t is_active = (mask >> (tid % 32)) & 1;

  // Write-Only
  // Рождение сигнала = сброс axon_heads[id] = 0
  if (is_active) {
    axon_heads[map_pixel_to_axon[tid]] = 0;
  }
}

extern "C" void launch_inject_inputs(uint32_t *axon_heads,
                                     const uint32_t *input_bitmask,
                                     const uint32_t *map_pixel_to_axon,
                                     uint32_t num_pixels,
                                     uint32_t tick_in_batch,
                                     void *stream) {
  int blockSize = 256;
  int numBlocks = (num_pixels + blockSize - 1) / blockSize;

  // Ensure we don't divide by zero if num_pixels is 0
  if (numBlocks > 0) {
    inject_inputs_kernel<<<numBlocks, blockSize, 0, (cudaStream_t)stream>>>(
        axon_heads, input_bitmask, map_pixel_to_axon, num_pixels, tick_in_batch);
  }
}
