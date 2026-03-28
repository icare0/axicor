use genesis_core::layout::{BurstHeads8, VariantParameters};
use crate::ffi::ShardVramPtrs;
use genesis_core::constants::AXON_SENTINEL;
use std::ffi::c_void;

// =============================================================================
// §1.1 Эмуляция Constant Memory (LUT)
// =============================================================================

#[repr(C, align(64))]
pub struct CpuConstantMemory {
    pub variants: [VariantParameters; 16],
}

pub static mut VARIANT_LUT: CpuConstantMemory = unsafe { std::mem::zeroed() };

pub unsafe fn cpu_upload_constant_memory(lut: *const VariantParameters) {
    let src = std::slice::from_raw_parts(lut, 16);
    std::ptr::copy_nonoverlapping(src.as_ptr(), std::ptr::addr_of_mut!(VARIANT_LUT.variants) as *mut VariantParameters, 16);
}

// =============================================================================
// §1.2 VRAM Allocation через libc
// =============================================================================

pub unsafe fn cpu_allocate_shard(
    padded_n: u32,
    total_axons: u32,
    out_vram: *mut ShardVramPtrs,
) -> i32 {
    let n = padded_n as usize;
    let total_state_size = n * 1166; 
    
    let mut base_ptr: *mut c_void = std::ptr::null_mut();
    let res = libc::posix_memalign(&mut base_ptr, 64, total_state_size);
    if res != 0 { return res; }
    
    std::ptr::write_bytes(base_ptr, 0, total_state_size);
    
    let base = base_ptr as *mut u8;
    
    (*out_vram).soma_voltage      = base.add(0) as *mut i32;
    (*out_vram).soma_flags        = base.add(4 * n) as *mut u8;
    (*out_vram).threshold_offset  = base.add(5 * n) as *mut i32;
    (*out_vram).timers            = base.add(9 * n) as *mut u8;
    (*out_vram).soma_to_axon      = base.add(10 * n) as *mut u32;
    (*out_vram).dendrite_targets  = base.add(14 * n) as *mut u32;
    (*out_vram).dendrite_weights  = base.add(526 * n) as *mut i32;
    (*out_vram).dendrite_timers   = base.add(1038 * n) as *mut u8;

    // DOD Fix: Initialize soma_to_axon with 0xFFFFFFFF (sentinel) 
    // to prevent multiple neurons from shifting axon heads at index 0.
    std::ptr::write_bytes((*out_vram).soma_to_axon as *mut u8, 0xFF, n * 4);
    
    let total_axons_size = total_axons as usize * std::mem::size_of::<BurstHeads8>();
    let mut axons_ptr: *mut c_void = std::ptr::null_mut();
    let res = libc::posix_memalign(&mut axons_ptr, 32, total_axons_size);
    if res != 0 {
        libc::free(base_ptr);
        return res;
    }
    
    let axon_slice = std::slice::from_raw_parts_mut(axons_ptr as *mut BurstHeads8, total_axons as usize);
    axon_slice.fill(BurstHeads8::empty(AXON_SENTINEL));
    
    (*out_vram).axon_heads = axons_ptr as *mut BurstHeads8;
    
    0
}

// =============================================================================
// §1.4 Zero-Copy DMA и Free
// =============================================================================

pub unsafe fn cpu_upload_state_blob(
    vram: *const ShardVramPtrs,
    state_blob: *const c_void,
    state_size: usize,
) -> i32 {
    std::ptr::copy_nonoverlapping(state_blob, (*vram).soma_voltage as *mut c_void, state_size);
    0
}

pub unsafe fn cpu_upload_axons_blob(
    vram: *const ShardVramPtrs,
    axons_blob: *const c_void,
    axons_size: usize,
) -> i32 {
    std::ptr::copy_nonoverlapping(axons_blob, (*vram).axon_heads as *mut c_void, axons_size);
    0
}

pub unsafe fn cpu_free_shard(vram: *mut ShardVramPtrs) {
    if vram.is_null() { return; }
    
    let ptrs = &mut *vram;
    
    if !ptrs.soma_voltage.is_null() {
        libc::free(ptrs.soma_voltage as *mut c_void);
    }
    
    if !ptrs.axon_heads.is_null() {
        libc::free(ptrs.axon_heads as *mut c_void);
    }
    
    // Architect Fix: Zero out the entire struct to prevent dangling pointers.
    std::ptr::write_bytes(vram, 0, 1);
}

// =============================================================================
// §1.5 Логика Тестирования
// =============================================================================

#[cfg(test)]
pub(crate) static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cu_shard_allocation_logic() {
        let mut ptrs: ShardVramPtrs = unsafe { std::mem::zeroed() };
        let padded_n = 64;
        let total_axons = 100;
        
        let res = unsafe { cpu_allocate_shard(padded_n, total_axons, &mut ptrs) };
        assert_eq!(res, 0);
        
        let base_addr = ptrs.soma_voltage as usize;
        assert_eq!(base_addr % 64, 0);
        
        let targets_addr = ptrs.dendrite_targets as usize;
        assert_eq!(targets_addr - base_addr, 14 * 64);
        
        unsafe {
            let first_head = *ptrs.axon_heads;
            assert_eq!(first_head.h0, AXON_SENTINEL);
        }
        
        unsafe { cpu_free_shard(&mut ptrs) };
        assert!(ptrs.soma_voltage.is_null());
        assert!(ptrs.axon_heads.is_null());
        assert!(ptrs.dendrite_targets.is_null());
    }

    #[test]
    fn test_constant_memory_layout() {
        assert_eq!(std::mem::size_of::<CpuConstantMemory>(), 1024);
        assert_eq!(std::mem::align_of::<CpuConstantMemory>(), 64);
    }
}
