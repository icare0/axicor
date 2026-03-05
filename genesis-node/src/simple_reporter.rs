use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub struct SimpleReporter {
    pub total_ticks: Arc<AtomicU64>,
    pub udp_out_packets: Arc<AtomicUsize>,
    pub start_time: Instant,
}

impl SimpleReporter {
    pub fn new() -> Self {
        Self {
            total_ticks: Arc::new(AtomicU64::new(0)),
            udp_out_packets: Arc::new(AtomicUsize::new(0)),
            start_time: Instant::now(),
        }
    }

    // Вызывается из отдельного потока
    pub fn print_status(&self) {
        let ticks = self.total_ticks.load(Ordering::Relaxed);
        let pkts = self.udp_out_packets.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed().as_secs_f64().max(0.001);
        
        let tps = ticks as f64 / elapsed;
        
        // \r переписывает текущую строку без выделения новой памяти терминалом
        eprint!(
            "\r[Genesis] Ticks: {:<8} | TPS: {:<7.0} | UDP Out: {:<8} | Uptime: {:.1} s", 
            ticks, tps, pkts, elapsed
        );
        use std::io::Write;
        let _ = std::io::stderr().flush();
    }
}
