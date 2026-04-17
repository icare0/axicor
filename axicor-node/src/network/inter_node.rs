use crate::network::SpikeEvent;
use std::ptr;

pub struct InterNodeChannel {
    pub target_zone_hash: u32,
    pub src_zone_hash: u32,
    pub capacity: u32, // [DOD FIX] Физический лимит VRAM
    pub count: u32,    // [DOD FIX] Текущее число активных связей

    pub src_indices_host: Vec<u32>,
    pub dst_ghost_ids_host: Vec<u32>, // [DOD FIX] Зеркало для Swap-and-Pop

    pub src_indices_d: *mut u32,
    pub dst_ghost_ids_d: *mut u32,

    // Zero-Copy Pinned RAM (доступен и GPU, и CPU)
    pub out_events_pinned: *mut SpikeEvent,
    pub out_count_pinned: *mut u32,
}

unsafe impl Send for InterNodeChannel {}
unsafe impl Sync for InterNodeChannel {}

impl Drop for InterNodeChannel {
    fn drop(&mut self) {
        unsafe {
            axicor_compute::ffi::gpu_free(self.src_indices_d as *mut _);
            axicor_compute::ffi::gpu_free(self.dst_ghost_ids_d as *mut _);
            axicor_compute::ffi::gpu_host_free(self.out_events_pinned as *mut _);
            axicor_compute::ffi::gpu_host_free(self.out_count_pinned as *mut _);
        }
    }
}

impl InterNodeChannel {
    pub unsafe fn new(src_zone_hash: u32, target_zone_hash: u32, src_indices: &[u32], dst_ghost_ids: &[u32], capacity: u32) -> Self {
        let count = src_indices.len() as u32;
        assert!(count <= capacity, "FATAL: Initial connections exceed capacity");

        // Аллокация под MAX CAPACITY, а не под текущий count
        let bytes_capacity = (capacity as usize) * 4;
        let src_d = axicor_compute::ffi::gpu_malloc(bytes_capacity) as *mut u32;
        let dst_d = axicor_compute::ffi::gpu_malloc(bytes_capacity) as *mut u32;

        // Копируем только РЕАЛЬНЫЕ данные
        if count > 0 {
            let bytes_active = (count as usize) * 4;
            axicor_compute::ffi::gpu_memcpy_host_to_device_async(src_d as *mut _, src_indices.as_ptr() as *const _, bytes_active, ptr::null_mut());
            axicor_compute::ffi::gpu_memcpy_host_to_device_async(dst_d as *mut _, dst_ghost_ids.as_ptr() as *const _, bytes_active, ptr::null_mut());
        }

        // Максимум 8 спайков на аксон за батч. Лимит берется от Capacity.
        let events_size = (capacity as usize) * 8 * std::mem::size_of::<SpikeEvent>();

        // Host-mirroring с преаллокацией
        let mut src_host = Vec::with_capacity(capacity as usize);
        src_host.extend_from_slice(src_indices);
        let mut dst_host = Vec::with_capacity(capacity as usize);
        dst_host.extend_from_slice(dst_ghost_ids);

        Self {
            target_zone_hash,
            src_zone_hash,
            capacity,
            count,
            src_indices_host: src_host,
            dst_ghost_ids_host: dst_host,
            src_indices_d: src_d,
            dst_ghost_ids_d: dst_d,
            out_events_pinned: axicor_compute::ffi::gpu_host_alloc(events_size) as *mut SpikeEvent,
            out_count_pinned: axicor_compute::ffi::gpu_host_alloc(4) as *mut u32,
        }
    }

    pub unsafe fn extract_spikes(&self, axon_heads: *const axicor_core::layout::BurstHeads8, sync_batch_ticks: u32, v_seg: u32, stream: axicor_compute::ffi::CudaStream) {
        if self.count == 0 { return; }
        axicor_compute::ffi::launch_extract_outgoing_spikes(
            axon_heads,
            self.src_indices_d,
            self.dst_ghost_ids_d,
            self.count, // Ядро читает только count, мусор в хвосте VRAM игнорируется
            sync_batch_ticks,
            v_seg,
            self.out_events_pinned as *mut std::ffi::c_void,
            self.out_count_pinned,
            stream
        );
    }

    /// O(1) добавление новой межзональной связи (Hot-Patching)
    pub unsafe fn push_route(&mut self, src_axon: u32, dst_ghost: u32, stream: axicor_compute::ffi::CudaStream) {
        assert!(self.count < self.capacity, "FATAL: Routing capacity exceeded. Increase ghost_capacity.");
        let idx = self.count as usize;

        self.src_indices_host.push(src_axon);
        self.dst_ghost_ids_host.push(dst_ghost);

        let src_ptr = self.src_indices_host.as_ptr().add(idx);
        let dst_ptr = self.dst_ghost_ids_host.as_ptr().add(idx);

        // Микро-DMA (4 байта) прямо в свободный хвост массива VRAM
        axicor_compute::ffi::gpu_memcpy_host_to_device_async(
            self.src_indices_d.add(idx) as *mut _,
            src_ptr as *const _,
            4,
            stream
        );
        axicor_compute::ffi::gpu_memcpy_host_to_device_async(
            self.dst_ghost_ids_d.add(idx) as *mut _,
            dst_ptr as *const _,
            4,
            stream
        );

        self.count += 1;
    }

    /// O(1) удаление связи через Swap-and-Pop
    pub unsafe fn prune_route(&mut self, target_ghost_id: u32, stream: axicor_compute::ffi::CudaStream) {
        // O(N) поиск на хосте допустим, т.к. выполняется только в Night Phase
        let Some(idx) = self.dst_ghost_ids_host.iter().position(|&g| g == target_ghost_id) else {
            return; // Связь уже удалена
        };

        let last_idx = (self.count - 1) as usize;

        // 1. Swap на хосте
        self.src_indices_host.swap(idx, last_idx);
        self.dst_ghost_ids_host.swap(idx, last_idx);

        // 2. Pop хвоста
        self.src_indices_host.pop();
        self.dst_ghost_ids_host.pop();

        // 3. Если удаленный элемент не был последним, нужно пропатчить образовавшуюся дыру в VRAM
        if idx != last_idx {
            let src_ptr = self.src_indices_host.as_ptr().add(idx);
            let dst_ptr = self.dst_ghost_ids_host.as_ptr().add(idx);

            axicor_compute::ffi::gpu_memcpy_host_to_device_async(
                self.src_indices_d.add(idx) as *mut _,
                src_ptr as *const _,
                4,
                stream
            );
            axicor_compute::ffi::gpu_memcpy_host_to_device_async(
                self.dst_ghost_ids_d.add(idx) as *mut _,
                dst_ptr as *const _,
                4,
                stream
            );
        }

        self.count -= 1;
    }
}
