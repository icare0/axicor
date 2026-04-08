use crate::memory::VramState;
use std::ptr;
use crate::cpu;

pub struct GpuIoBuffers {
    pub d_input_bitmask: *mut u32,
    pub d_incoming_spikes: *mut u32,
    pub d_output_history: *mut u8,
}

pub struct CpuIoBuffers {
    pub h_input_bitmask: Vec<u32>,
    pub h_incoming_spikes: Vec<u32>,
    pub h_output_history: Vec<u8>,
}

pub enum IoBackend {
    Gpu(GpuIoBuffers),
    Cpu(CpuIoBuffers),
}

pub struct IoBuffers {
    pub backend: IoBackend,
    pub max_spikes_per_tick: u32,
    pub input_words_per_tick: u32,
    pub num_outputs: u32,
}

unsafe impl Send for IoBuffers {}
unsafe impl Sync for IoBuffers {}

pub struct GpuEngine {
    pub vram: VramState,
    pub stream: crate::ffi::CudaStream,
    pub telemetry_ids_d: *mut u32,
    pub telemetry_count_d: *mut u32,
    pub telemetry_count_pinned_h: *mut u32,
}

pub struct CpuEngine {
    pub vram: VramState,
    pub telemetry_ids: Vec<u32>,
    pub telemetry_count: u32,
}

pub enum ShardEngine {
    Gpu(GpuEngine),
    Cpu(CpuEngine),
}

unsafe impl Send for ShardEngine {}
unsafe impl Sync for ShardEngine {}

impl ShardEngine {
    pub fn new(vram: VramState) -> Self {
        let n = vram.padded_n as usize;
        if vram.use_gpu {
            #[cfg_attr(feature = "mock-gpu", allow(unused_mut))]
            let mut stream = std::ptr::null_mut();
            #[cfg(not(feature = "mock-gpu"))]
            {
                let err = unsafe { crate::ffi::gpu_stream_create(&mut stream) };
                assert_eq!(err, 0, "FATAL: cudaStreamCreate failed");
            }

            let telemetry_ids_d: *mut u32;
            let telemetry_count_d: *mut u32;
            let telemetry_count_pinned_h: *mut u32;

            unsafe {
                telemetry_ids_d = crate::ffi::gpu_malloc(n * 4) as *mut u32;
                telemetry_count_d = crate::ffi::gpu_malloc(4) as *mut u32;
                telemetry_count_pinned_h = crate::ffi::gpu_host_alloc(4) as *mut u32;
            }

            Self::Gpu(GpuEngine { vram, stream, telemetry_ids_d, telemetry_count_d, telemetry_count_pinned_h })
        } else {
            Self::Cpu(CpuEngine {
                vram,
                telemetry_ids: vec![0u32; n],
                telemetry_count: 0,
            })
        }
    }

    pub fn step_day_phase_batch(
        &mut self,
        sync_batch_ticks: u32,
        io_buffers: &mut IoBuffers,
        h_input_bitmask: Option<&[u32]>,
        h_incoming_spikes: Option<&[u32]>,
        h_spike_counts: &[u32],
        virtual_offset: u32,
        num_virtual_axons: u32,
        mapped_soma_ids_device: *const u32, // Device ptr for GPU, Host ptr for CPU
        v_seg: u32,
        dopamine: i16,
        tick_base: u32,
    ) {
        match self {
            Self::Gpu(ref mut gpu) => {
                let total_input_words = io_buffers.input_words_per_tick * sync_batch_ticks;
                let total_schedule_capacity = io_buffers.max_spikes_per_tick * sync_batch_ticks;

                if let IoBackend::Gpu(ref mut b) = io_buffers.backend {
                    unsafe {
                        crate::ffi::cu_dma_h2d_io(
                            b.d_input_bitmask,
                            h_input_bitmask.map_or(ptr::null(), |s| s.as_ptr()),
                            if h_input_bitmask.is_some() { total_input_words } else { 0 },
                            b.d_incoming_spikes,
                            h_incoming_spikes.map_or(ptr::null(), |s| s.as_ptr()),
                            if h_incoming_spikes.is_some() { total_schedule_capacity } else { 0 },
                            gpu.stream,
                        );
                        
                        crate::ffi::cu_reset_burst_counters(&gpu.vram.ptrs, gpu.vram.padded_n, gpu.stream);
                        crate::ffi::gpu_reset_telemetry_count(gpu.telemetry_count_d, gpu.stream);

                        for tick in 0..sync_batch_ticks {
                            let global_tick = tick_base + tick;
                            let tick_input_ptr = if b.d_input_bitmask.is_null() { ptr::null() } else { b.d_input_bitmask.add((tick * io_buffers.input_words_per_tick) as usize) };
                            let tick_spikes_ptr = if b.d_incoming_spikes.is_null() { ptr::null() } else { b.d_incoming_spikes.add((tick * io_buffers.max_spikes_per_tick) as usize) };
                            let tick_output_ptr = if b.d_output_history.is_null() { ptr::null_mut() } else { b.d_output_history.add((tick * io_buffers.num_outputs) as usize) };

                            crate::ffi::cu_step_day_phase(
                                &gpu.vram.ptrs, gpu.vram.padded_n, gpu.vram.total_axons, v_seg, global_tick,
                                tick_input_ptr, virtual_offset, num_virtual_axons,
                                tick_spikes_ptr, h_spike_counts[tick as usize],
                                mapped_soma_ids_device,
                                tick_output_ptr, io_buffers.num_outputs, dopamine, gpu.stream
                            );

                            crate::ffi::launch_extract_telemetry(gpu.vram.ptrs.soma_flags, gpu.telemetry_ids_d, gpu.telemetry_count_d, gpu.vram.padded_n, gpu.stream);
                        }
                    }
                }
            }
            Self::Cpu(ref mut cpu) => {
                if let IoBackend::Cpu(ref mut b) = io_buffers.backend {
                    for tick in 0..sync_batch_ticks {
                        let global_tick = tick_base + tick;
                        let tick_idx = tick as usize;

                        // 0. Inject Virtual Axons (Sensors)
                        if let Some(mask) = h_input_bitmask {
                            let start = tick_idx * io_buffers.input_words_per_tick as usize;
                            let end = start.saturating_add(io_buffers.input_words_per_tick as usize);
                            if end <= mask.len() {
                                let tick_mask = &mask[start..end];
                                unsafe {
                                    let axon_heads = std::slice::from_raw_parts_mut(cpu.vram.ptrs.axon_heads, cpu.vram.total_axons as usize);
                                    crate::cpu::physics::cpu_inject_inputs(axon_heads, tick_mask, virtual_offset, num_virtual_axons, v_seg);
                                }
                            }
                        }
                        
                        // 1. Inject Network Spikes
                        if let Some(spikes) = h_incoming_spikes {
                            let start = tick_idx * io_buffers.max_spikes_per_tick as usize;
                            let count = h_spike_counts
                                .get(tick_idx)
                                .copied()
                                .unwrap_or(0)
                                .min(io_buffers.max_spikes_per_tick) as usize;
                            let end = start.saturating_add(count);
                            if end <= spikes.len() {
                                let tick_spikes = &spikes[start..end];

                                unsafe {
                                    let axon_heads = std::slice::from_raw_parts_mut(cpu.vram.ptrs.axon_heads, cpu.vram.total_axons as usize);
                                    cpu::physics::cpu_apply_spike_batch(axon_heads, tick_spikes, v_seg);
                                }
                            }
                        }
                        
                        unsafe {
                            let axon_heads = std::slice::from_raw_parts_mut(cpu.vram.ptrs.axon_heads, cpu.vram.total_axons as usize);
                            cpu::physics::cpu_propagate_axons(axon_heads, v_seg);
                        }

                        // 2. GLIF Physics
                        unsafe {
                            cpu::physics::cpu_update_neurons(&cpu.vram.ptrs, cpu.vram.padded_n, global_tick, v_seg);
                        }

                        // 3. Plasticity
                        unsafe {
                            cpu::physics::cpu_apply_gsop(&cpu.vram.ptrs, cpu.vram.padded_n, dopamine);
                        }

                        // 4. Output History
                        let soma_flags = unsafe { std::slice::from_raw_parts(cpu.vram.ptrs.soma_flags, cpu.vram.padded_n as usize) };
                        let mapped_ids = unsafe { std::slice::from_raw_parts(mapped_soma_ids_device, io_buffers.num_outputs as usize) };
                        cpu::physics::cpu_record_outputs(soma_flags, mapped_ids, &mut b.h_output_history, tick, io_buffers.num_outputs);

                        // 5. Telemetry
                        cpu.telemetry_count = cpu::physics::cpu_extract_telemetry(soma_flags, &mut cpu.telemetry_ids);
                    }
                }
            }
        }
    }
}

impl Drop for ShardEngine {
    fn drop(&mut self) {
        match self {
            Self::Gpu(_gpu) => {
                #[cfg(not(feature = "mock-gpu"))]
                unsafe {
                    crate::ffi::gpu_stream_destroy(_gpu.stream);
                    crate::ffi::gpu_free(_gpu.telemetry_ids_d as *mut _);
                    crate::ffi::gpu_free(_gpu.telemetry_count_d as *mut _);
                    crate::ffi::gpu_host_free(_gpu.telemetry_count_pinned_h as *mut _);
                }
            }
            _ => {}
        }
    }
}
