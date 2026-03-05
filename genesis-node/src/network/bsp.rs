use crate::network::ring_buffer::SpikeSchedule;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// BSP Барьер для синхронизации сети и вычислителя (Latency Hiding).
/// Мы используем Ping-Pong Double Buffering: пока GPU читает из A, сеть пишет в B.
pub struct BspBarrier {
    pub schedule_a: SpikeSchedule,
    pub schedule_b: SpikeSchedule,
    /// Если true, UDP-сервер пишет в B, а GPU читает из A.
    pub writing_to_b: AtomicBool, 
    // [DOD] Сетевая синхронизация
    pub expected_peers: usize,
    pub packets_received: AtomicUsize,
}

impl BspBarrier {
    pub fn new(sync_batch_ticks: usize, expected_peers: usize) -> Self {
        Self {
            schedule_a: SpikeSchedule::new(sync_batch_ticks),
            schedule_b: SpikeSchedule::new(sync_batch_ticks),
            writing_to_b: AtomicBool::new(true),
            expected_peers,
            packets_received: AtomicUsize::new(0),
        }
    }

    pub fn wait_for_data_sync(&self) {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(50); // Ждем максимум 50мс

        // Ждем, пока Ingress UDP-сервер не запишет пакеты от всех соседей
        while self.packets_received.load(Ordering::Acquire) < self.expected_peers {
            if start.elapsed() > timeout {
                println!("⚠️ [BSP] UDP Drop Detected! Forcing barrier bypass to prevent deadlock.");
                break;
            }
            // [DOD] Выжигаем токены CPU минимально, не отдавая тред ОС
            std::hint::spin_loop();
        }
    }

    /// Вызывается ядром Node в конце батча: меняет буферы местами.
    /// Переключение происходит на барьере BSP, гарантируя отсутствие гонок за данные (Race Conditions).
    pub fn sync_and_swap(&self) {
        // Сбрасываем барьер для следующего тика
        self.packets_received.store(0, Ordering::Release);
        
        let was_b = self.writing_to_b.fetch_xor(true, Ordering::SeqCst);
        if was_b {
            // Сеть закончила писать в B, теперь мы сделаем B доступным для чтения GPU.
            // Старый буфер A (который читал GPU) теперь свободен для записи.
            self.schedule_a.clear();
        } else {
            self.schedule_b.clear();
        }
    }

    /// Возвращает ссылку на буфер, в который сейчас должна писать сеть (Tokio).
    pub fn get_write_schedule(&self) -> &SpikeSchedule {
        if self.writing_to_b.load(Ordering::Acquire) {
            &self.schedule_b
        } else {
            &self.schedule_a
        }
    }

    /// Возвращает ссылку на буфер, из которого сейчас должен читать GPU (genesis-compute).
    pub fn get_read_schedule(&self) -> &SpikeSchedule {
        if self.writing_to_b.load(Ordering::Acquire) {
            &self.schedule_a
        } else {
            &self.schedule_b
        }
    }
}
