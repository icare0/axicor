// axicor-baker/src/bake/input_map.rs
//
// Phase A: Input Matrix / Virtual Axons (GXI)
// Specification: 08_io_matrix.md 2.1 / 09_baking_pipeline.md 2.1

use axicor_core::config::io::IoConfig;
use axicor_core::constants::GXI_MAGIC;
use axicor_core::hash::fnv1a_32;
use std::io::Write;
use std::path::Path;

/// Descriptor of a single matrix in a .gxi file (16 bytes)
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GxiMatrixDescriptor {
    pub name_hash: u32,
    pub offset: u32, // Index in Axon Array
    pub width: u16,
    pub height: u16,
    pub stride: u8,
    pub _padding: [u8; 3],
}

/// Result of baking a single input matrix.
#[derive(Clone)]
pub struct BakedGxi {
    pub name_hash: u32,
    pub width: u16,
    pub height: u16,
    pub stride: u8,
    pub axon_ids: Vec<u32>,
}

pub fn build_gxi_mappings(
    io_config: &IoConfig,
    _zone_name: &str,
    mut base_axon_id: u32,
) -> Vec<BakedGxi> {
    let mut gxi_matrices = Vec::new();

    for matrix in &io_config.input {
        for pin in &matrix.pin {
            let total_pixels = pin.width * pin.height;
            let axon_ids: Vec<u32> = (0..total_pixels).map(|i| base_axon_id + i).collect();

            gxi_matrices.push(BakedGxi {
                name_hash: fnv1a_32(pin.name.as_bytes()), // PIN-BASED ROUTING
                width: pin.width as u16,
                height: pin.height as u16,
                stride: pin.stride as u8,
                axon_ids,
            });

            base_axon_id += total_pixels;
        }
    }
    gxi_matrices
}

pub fn write_gxi_file(out_dir: &Path, matrices: &[BakedGxi]) {
    let path = out_dir.join("shard.gxi");
    let mut file = std::fs::File::create(path).expect("Failed to create .gxi file");

    let total_pixels: u32 = matrices.iter().map(|m| m.axon_ids.len() as u32).sum();
    let num_matrices = matrices.len() as u16;

    file.write_all(&GXI_MAGIC.to_le_bytes()).unwrap();
    file.write_all(&[1u8, 0u8]).unwrap();
    file.write_all(&num_matrices.to_le_bytes()).unwrap();
    file.write_all(&total_pixels.to_le_bytes()).unwrap();

    let mut current_offset = 0;
    for m in matrices {
        let desc = GxiMatrixDescriptor {
            name_hash: m.name_hash,
            offset: current_offset,
            width: m.width,
            height: m.height,
            stride: m.stride,
            _padding: [0; 3],
        };
        unsafe {
            let bytes = std::slice::from_raw_parts(
                (&desc as *const GxiMatrixDescriptor) as *const u8,
                std::mem::size_of::<GxiMatrixDescriptor>(),
            );
            file.write_all(bytes).unwrap();
        }
        current_offset += m.axon_ids.len() as u32;
    }

    for m in matrices {
        let payload_bytes = unsafe {
            std::slice::from_raw_parts(
                m.axon_ids.as_ptr() as *const u8,
                m.axon_ids.len() * std::mem::size_of::<u32>(),
            )
        };
        file.write_all(payload_bytes)
            .expect("Failed to write axon IDs");
    }
}
