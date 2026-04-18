use axicor_compute::memory::PinnedBuffer;
use std::sync::atomic::{AtomicU32, Ordering};

/// Maximum spikes per tick within a batch.
/// [LAW]: This is the limit for Ghost-traffic to prevent VRAM overflow.
const MAX_SPIKES_PER_TICK: usize = 100_000;

/// Flat spike schedule for GPU dispatch (Zero-Copy).
/// We don't use Priority Queue as GPU reads data sequentially.
pub struct SpikeSchedule {
    pub sync_batch_ticks: usize,
    /// Spike count per tick (length = sync_batch_ticks).
    pub counts: PinnedBuffer<AtomicU32>,
    /// Flat ID buffer (length = sync_batch_ticks * MAX_SPIKES_PER_TICK).
    /// This array is copied directly to Pinned Memory for DMA.
    pub ghost_ids: PinnedBuffer<u32>,
}

unsafe impl Send for SpikeSchedule {}
unsafe impl Sync for SpikeSchedule {}

impl SpikeSchedule {
    pub fn new(sync_batch_ticks: usize) -> Self {
        Self {
            sync_batch_ticks,
            counts: PinnedBuffer::new(sync_batch_ticks)
                .expect("PinnedBuffer alloc failed for counts"),
            ghost_ids: PinnedBuffer::new(sync_batch_ticks * MAX_SPIKES_PER_TICK)
                .expect("PinnedBuffer alloc failed for ghost_ids"),
        }
    }

    /// O(1) network spike insertion. Lock-free slot reservation.
    /// tick_offset is relative to start of current BSP batch.
    #[inline(always)]
    pub fn push_spike(&self, tick_offset: usize, ghost_id: u32) {
        if tick_offset >= self.sync_batch_ticks {
            return; // Out-of-Bounds protection (drop anomalies)
        }

        // Lock-free slot reservation via 1 XADD instruction
        let count = self.counts.as_slice()[tick_offset].fetch_add(1, Ordering::AcqRel) as usize;

        if count < MAX_SPIKES_PER_TICK {
            // Each thread has unique count, no write conflict!
            let idx = (tick_offset * MAX_SPIKES_PER_TICK) + count;
            unsafe {
                let ptr = self.ghost_ids.as_ptr() as *mut u32;
                *ptr.add(idx) = ghost_id;
            }
        } else {
            // Rollback counter on overflow
            self.counts.as_slice()[tick_offset].fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Reset after GPU consumed the batch.
    pub fn clear(&self) {
        // Fast zero-reset before handing to network
        for c in self.counts.as_slice() {
            c.store(0, Ordering::Relaxed);
        }
    }
}
