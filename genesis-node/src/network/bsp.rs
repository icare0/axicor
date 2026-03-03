use crate::network::ring_buffer::SpikeSchedule;

/// BSP Барьер для синхронизации сети и вычислителя (Latency Hiding).
/// Мы используем Ping-Pong Double Buffering: пока GPU читает из A, сеть пишет в B.
pub struct BspBarrier {
    pub schedule_a: SpikeSchedule,
    pub schedule_b: SpikeSchedule,
    /// Если true, UDP-сервер пишет в B, а GPU читает из A.
    pub writing_to_b: bool, 
}

impl BspBarrier {
    pub fn new(sync_batch_ticks: usize) -> Self {
        Self {
            schedule_a: SpikeSchedule::new(sync_batch_ticks),
            schedule_b: SpikeSchedule::new(sync_batch_ticks),
            writing_to_b: true,
        }
    }

    /// Вызывается ядром Node в конце батча: меняет буферы местами.
    /// Переключение происходит на барьере BSP, гарантируя отсутствие гонок за данные (Race Conditions).
    pub fn sync_and_swap(&mut self) {
        if self.writing_to_b {
            // Сеть закончила писать в B, теперь мы сделаем B доступным для чтения GPU.
            // Старый буфер A (который читал GPU) теперь свободен для записи.
            self.schedule_a.clear();
        } else {
            self.schedule_b.clear();
        }
        self.writing_to_b = !self.writing_to_b;
    }

    /// Возвращает ссылку на буфер, в который сейчас должна писать сеть (Tokio).
    pub fn get_write_schedule(&mut self) -> &mut SpikeSchedule {
        if self.writing_to_b {
            &mut self.schedule_b
        } else {
            &mut self.schedule_a
        }
    }

    /// Возвращает ссылку на буфер, из которого сейчас должен читать GPU (genesis-compute).
    pub fn get_read_schedule(&self) -> &SpikeSchedule {
        if self.writing_to_b {
            &self.schedule_a
        } else {
            &self.schedule_b
        }
    }
}
