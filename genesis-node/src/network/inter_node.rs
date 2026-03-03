use genesis_compute::ffi;
use crate::network::SpikeEvent;
use std::ptr;

pub struct InterNodeChannel {
    pub target_zone_hash: u32,
    pub src_indices_host: Vec<u32>,
    pub src_indices_d: *mut u32,
    pub dst_ghost_ids_d: *mut u32,
    pub count: u32,
    
    // Zero-Copy Pinned RAM (доступен и GPU, и CPU)
    pub out_events_pinned: *mut SpikeEvent,
    pub out_count_pinned: *mut u32,
}

unsafe impl Send for InterNodeChannel {}
unsafe impl Sync for InterNodeChannel {}

impl Clone for InterNodeChannel {
    fn clone(&self) -> Self {
        Self {
            target_zone_hash: self.target_zone_hash,
            src_indices_host: self.src_indices_host.clone(),
            src_indices_d: self.src_indices_d,
            dst_ghost_ids_d: self.dst_ghost_ids_d,
            count: self.count,
            out_events_pinned: self.out_events_pinned,
            out_count_pinned: self.out_count_pinned,
        }
    }
}

impl InterNodeChannel {
    pub unsafe fn new(target_zone_hash: u32, src_indices: &[u32], dst_ghost_ids: &[u32]) -> Self {
        let count = src_indices.len() as u32;
        
        let src_d = ffi::gpu_malloc((count as usize) * 4) as *mut u32;
        let dst_d = ffi::gpu_malloc((count as usize) * 4) as *mut u32;
        ffi::gpu_memcpy_host_to_device_async(src_d as *mut _, src_indices.as_ptr() as *const _, (count as usize) * 4, ptr::null_mut());
        ffi::gpu_memcpy_host_to_device_async(dst_d as *mut _, dst_ghost_ids.as_ptr() as *const _, (count as usize) * 4, ptr::null_mut());

        // Максимум 1 спайк на аксон за батч
        // Используем 8 байт (SpikeEvent pack layout)
        let events_size = (count as usize) * std::mem::size_of::<SpikeEvent>();
        
        Self {
            target_zone_hash,
            src_indices_host: src_indices.to_vec(),
            src_indices_d: src_d,
            dst_ghost_ids_d: dst_d,
            count,
            out_events_pinned: ffi::gpu_host_alloc(events_size) as *mut SpikeEvent,
            out_count_pinned: ffi::gpu_host_alloc(4) as *mut u32,
        }
    }

    pub unsafe fn extract_spikes(&self, axon_heads: *const u32, sync_batch_ticks: u32, stream: ffi::CudaStream) {
        if self.count == 0 { return; }
        genesis_compute::ffi::launch_extract_outgoing_spikes(
            axon_heads,
            self.src_indices_d,
            self.dst_ghost_ids_d,
            self.count,
            sync_batch_ticks,
            self.out_events_pinned as *mut std::ffi::c_void,
            self.out_count_pinned,
            stream
        );
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpikeBatchHeaderV2 {
    pub src_zone_hash: u64,
    pub dst_zone_hash: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpikeEventV2 {
    pub ghost_id: u32,
    pub tick_offset: u32, // Выровнено до 8 байт для Coalesced Access
}

pub struct InterNodeRouter;

impl InterNodeRouter {
    /// Запускает слушатель межзональных спайков (Sender-Side Mapping)
    pub async fn spawn_ghost_listener(
        port: u16,
        tx: tokio::sync::mpsc::UnboundedSender<(SpikeBatchHeaderV2, Vec<SpikeEventV2>)>,
    ) {
        let sock = tokio::net::UdpSocket::bind(("0.0.0.0", port)).await.expect("FATAL: Ghost Bind failed");
        
        tokio::spawn(async move {
            let mut buf = vec![0u8; 65507];
            loop {
                if let Ok((size, _addr)) = sock.recv_from(&mut buf).await {
                    if size < 16 { continue; }
                    
                    let header: SpikeBatchHeaderV2 = *bytemuck::from_bytes(&buf[0..16]);
                    let payload_bytes = &buf[16..size];
                    
                    if payload_bytes.len() % 8 != 0 { continue; }
                    
                    let events: &[SpikeEventV2] = bytemuck::cast_slice(payload_bytes);
                    
                    if tx.send((header, events.to_vec())).is_err() {
                        break;
                    }
                }
            }
        });
    }
}
