use std::time::Instant;
use std::collections::VecDeque;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase { Day, Night, Sleep }

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LogLevel { Info, Warning, Night, Error }

pub struct LogEntry {
    pub timestamp: String,
    pub message: String,
    pub level: LogLevel,
}

pub struct ZoneMetrics {
    pub hash: u32,
    pub name: String,
    pub short_name: String,
    pub neuron_count: u32,
    pub axon_count: u32,
    pub spikes_last_batch: u32,
    pub spike_rate: f64,
    pub phase: Phase,
    pub night_interval_ticks: u64,
}

pub struct DashboardState {
    pub is_running: bool,
    
    // Core Loop
    pub batch_number: u64,
    pub total_ticks: u64,
    pub uptime: Instant,
    pub wall_ms_history: VecDeque<u64>, // changed from f64 to u64 to suit ratatui sparkline better (or keeping u64 ms is simpler)
    pub ticks_per_sec: f64,

    // Per-Zone
    pub zones: Vec<ZoneMetrics>,

    // I/O
    pub udp_in_packets: u64,
    pub udp_out_packets: u64,
    pub oversized_skips: u64,
    pub connected_clients: u32,

    // VRAM
    pub vram_used_mb: f64,
    pub vram_total_mb: f64,
    pub vram_somas_mb: f64,
    pub vram_dendrites_mb: f64,
    pub vram_axons_mb: f64,

    // Night Phase
    pub night_count: u32,
    pub night_interval_ticks: u64,
    pub global_phase: Phase,

    // Event Log
    pub events: VecDeque<LogEntry>,

    // UI State
    pub zone_scroll: usize,
    pub log_scroll: usize,
    pub selected_zone_idx: Option<usize>,
}

impl DashboardState {
    pub fn new() -> Self {
        Self {
            is_running: true,
            batch_number: 0,
            total_ticks: 0,
            uptime: Instant::now(),
            wall_ms_history: VecDeque::with_capacity(60),
            ticks_per_sec: 0.0,
            zones: Vec::new(),
            udp_in_packets: 0,
            udp_out_packets: 0,
            oversized_skips: 0,
            connected_clients: 0,
            vram_used_mb: 0.0,
            vram_total_mb: 0.0,
            vram_somas_mb: 0.0,
            vram_dendrites_mb: 0.0,
            vram_axons_mb: 0.0,
            night_count: 0,
            night_interval_ticks: 200_000,
            global_phase: Phase::Day,
            events: VecDeque::with_capacity(200),
            zone_scroll: 0,
            log_scroll: 0,
            selected_zone_idx: None,
        }
    }

    pub fn push_log(&mut self, message: String, level: LogLevel) {
        let now = chrono::Local::now();
        let timestamp = now.format("%H:%M:%S").to_string();
        
        if self.events.len() >= 200 {
            self.events.pop_front();
        }
        self.events.push_back(LogEntry { timestamp, message, level });
    }

    pub fn push_wall_ms(&mut self, ms: u64) {
        if self.wall_ms_history.len() >= 60 {
            self.wall_ms_history.pop_front();
        }
        self.wall_ms_history.push_back(ms);
    }
}
