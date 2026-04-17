use axicor_compute::memory::PinnedBuffer;
use std::sync::atomic::{AtomicU32, Ordering};

/// Максимальное количество спайков на один тик внутри батча.
/// [ЗАКОН]: Это лимит для Ghost-трафика, чтобы предотвратить переполнение VRAM.
const MAX_SPIKES_PER_TICK: usize = 100_000;

/// Плоское расписание спайков для отправки в GPU (Zero-Copy).
/// Мы не используем Priority Queue, так как GPU читает данные последовательно.
pub struct SpikeSchedule {
    pub sync_batch_ticks: usize,
    /// Количество спайков на каждый тик (длина = sync_batch_ticks).
    pub counts: PinnedBuffer<AtomicU32>,
    /// Плоский буфер ID (длина = sync_batch_ticks * MAX_SPIKES_PER_TICK).
    /// Этот массив напрямую копируется в Pinned Memory для DMA.
    pub ghost_ids: PinnedBuffer<u32>,
}

unsafe impl Send for SpikeSchedule {}
unsafe impl Sync for SpikeSchedule {}

impl SpikeSchedule {
    pub fn new(sync_batch_ticks: usize) -> Self {
        Self {
            sync_batch_ticks,
            counts: PinnedBuffer::new(sync_batch_ticks).expect("PinnedBuffer alloc failed for counts"),
            ghost_ids: PinnedBuffer::new(sync_batch_ticks * MAX_SPIKES_PER_TICK)
                .expect("PinnedBuffer alloc failed for ghost_ids"),
        }
    }

    /// O(1) вставка спайка из сети. Lock-free резервация слотов.
    /// tick_offset — это смещение относительно начала текущего BSP-батча.
    #[inline(always)]
    pub fn push_spike(&self, tick_offset: usize, ghost_id: u32) {
        if tick_offset >= self.sync_batch_ticks {
            return; // Защита от Out-of-Bounds (отбрасываем аномалии)
        }

        // Lock-free резервация слота за 1 инструкцию XADD
        let count = self.counts.as_slice()[tick_offset].fetch_add(1, Ordering::AcqRel) as usize;
        
        if count < MAX_SPIKES_PER_TICK {
            // У каждого потока свой уникальный count, конфликта записи нет!
            let idx = (tick_offset * MAX_SPIKES_PER_TICK) + count;
            unsafe {
                let ptr = self.ghost_ids.as_ptr() as *mut u32;
                *ptr.add(idx) = ghost_id;
            }
        } else {
            // Откат счетчика при переполнении
            self.counts.as_slice()[tick_offset].fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Сброс после того, как GPU потребил батч.
    pub fn clear(&self) {
        // Быстрый сброс нулями перед передачей сети
        for c in self.counts.as_slice() {
            c.store(0, Ordering::Relaxed);
        }
    }
}
