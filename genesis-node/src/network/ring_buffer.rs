/// Максимальное количество спайков на один тик внутри батча.
/// [ЗАКОН]: Это лимит для Ghost-трафика, чтобы предотвратить переполнение VRAM.
const MAX_SPIKES_PER_TICK: usize = 100_000;

/// Плоское расписание спайков для отправки в GPU (Zero-Copy).
/// Мы не используем Priority Queue, так как GPU читает данные последовательно.
pub struct SpikeSchedule {
    pub sync_batch_ticks: usize,
    /// Количество спайков на каждый тик (длина = sync_batch_ticks).
    pub counts: Vec<u32>,
    /// Плоский буфер ID (длина = sync_batch_ticks * MAX_SPIKES_PER_TICK).
    /// Этот массив напрямую копируется в Pinned Memory для DMA.
    pub ghost_ids: Vec<u32>,
}

impl SpikeSchedule {
    pub fn new(sync_batch_ticks: usize) -> Self {
        Self {
            sync_batch_ticks,
            counts: vec![0; sync_batch_ticks],
            ghost_ids: vec![0; sync_batch_ticks * MAX_SPIKES_PER_TICK],
        }
    }

    /// O(1) вставка спайка из сети.
    /// tick_offset — это смещение относительно начала текущего BSP-батча.
    #[inline(always)]
    pub fn push_spike(&mut self, tick_offset: usize, ghost_id: u32) {
        if tick_offset >= self.sync_batch_ticks {
            return; // Защита от Out-of-Bounds (отбрасываем аномалии)
        }

        let count = self.counts[tick_offset] as usize;
        if count < MAX_SPIKES_PER_TICK {
            let idx = (tick_offset * MAX_SPIKES_PER_TICK) + count;
            self.ghost_ids[idx] = ghost_id;
            self.counts[tick_offset] += 1;
        } else {
            // В будущем здесь будет инкремент логаDroppedSpikes
        }
    }

    /// Сброс после того, как GPU потребил батч.
    pub fn clear(&mut self) {
        self.counts.fill(0);
        // ghost_ids обнулять не нужно, они просто перезаписываются новыми данными [DOD]
    }
}
