// axicor-baker/src/bake/output_map.rs
//
// Phase B: Readout Interface (GXO)
// Specification: 08_io_matrix.md 3.1 / 09_baking_pipeline.md 2.2

use axicor_core::config::io::IoConfig;
use axicor_core::constants::GXO_MAGIC;
use axicor_core::hash::fnv1a_32;
use axicor_core::ipc::EMPTY_PIXEL;
use std::path::Path;

/// Descriptor of a single matrix in a .gxo file (16 bytes)
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GxoMatrixDescriptor {
    pub name_hash: u32,
    pub offset: u32, // Index into Soma Array
    pub width: u16,
    pub height: u16,
    pub stride: u8,
    pub _padding: [u8; 3],
}

/// Baking result of a single pin (virtual matrix for the GPU).
#[derive(Clone)]
pub struct BakedGxo {
    pub name_hash: u32,
    pub width: u16,
    pub height: u16,
    pub stride: u8,
    pub mapped_soma_ids: Vec<u32>,
}

pub fn build_gxo_mappings(
    io_config: &IoConfig,
    _zone_name: &str,
    zone_width_vox: u32,
    zone_depth_vox: u32,
    neurons_packed_pos: &[u32],
    type_names: &[String],
) -> Vec<BakedGxo> {
    let mut gxo_matrices = Vec::new();

    for matrix in &io_config.output {
        for pin in &matrix.pin {
            let target_type_id = if pin.target_type.is_empty() || pin.target_type == "All" {
                None
            } else {
                type_names
                    .iter()
                    .position(|n| n == &pin.target_type)
                    .map(|id| id as u8)
            };

            let total_pixels = (pin.width * pin.height) as usize;
            let mut mapped_soma_ids = vec![EMPTY_PIXEL; total_pixels];
            let mut min_z_per_pixel = vec![u32::MAX; total_pixels];

            for (dense_id, &packed) in neurons_packed_pos.iter().enumerate() {
                if packed == 0 {
                    continue;
                }
                let p_struct = axicor_core::types::PackedPosition(packed);

                if let Some(target) = target_type_id {
                    if p_struct.type_id() != target {
                        continue;
                    }
                }

                // Inverse UV Projection
                let u_vox = p_struct.x() as f32 / zone_width_vox.max(1) as f32;
                let v_vox = p_struct.y() as f32 / zone_depth_vox.max(1) as f32;

                // AABB Check (Pin boundaries)
                if u_vox < pin.local_u
                    || u_vox >= pin.local_u + pin.u_width
                    || v_vox < pin.local_v
                    || v_vox >= pin.local_v + pin.v_height
                {
                    continue;
                }

                let local_u_in_pin = (u_vox - pin.local_u) / pin.u_width;
                let local_v_in_pin = (v_vox - pin.local_v) / pin.v_height;

                let px =
                    ((local_u_in_pin * pin.width as f32) as u32).min(pin.width.saturating_sub(1));
                let py =
                    ((local_v_in_pin * pin.height as f32) as u32).min(pin.height.saturating_sub(1));
                let pixel_idx = (py * pin.width + px) as usize;

                let vz = p_struct.z() as u32;
                if vz < min_z_per_pixel[pixel_idx] {
                    min_z_per_pixel[pixel_idx] = vz;
                    mapped_soma_ids[pixel_idx] = dense_id as u32;
                }
            }

            gxo_matrices.push(BakedGxo {
                name_hash: fnv1a_32(pin.name.as_bytes()),
                width: pin.width as u16,
                height: pin.height as u16,
                stride: pin.stride as u8,
                mapped_soma_ids,
            });
        }
    }
    gxo_matrices
}

pub fn write_gxo_file(out_dir: &Path, matrices: &[BakedGxo]) {
    use std::io::Write;
    let path = out_dir.join("shard.gxo");
    let mut file = std::fs::File::create(path).expect("Failed to create .gxo file");

    let total_pixels: u32 = matrices
        .iter()
        .map(|m| m.mapped_soma_ids.len() as u32)
        .sum();
    let num_matrices = matrices.len() as u16;

    file.write_all(&GXO_MAGIC.to_le_bytes()).unwrap();
    file.write_all(&[1u8, 0u8]).unwrap();
    file.write_all(&num_matrices.to_le_bytes()).unwrap();
    file.write_all(&total_pixels.to_le_bytes()).unwrap();

    let mut current_offset = 0;
    for m in matrices {
        let desc = GxoMatrixDescriptor {
            name_hash: m.name_hash,
            offset: current_offset,
            width: m.width,
            height: m.height,
            stride: m.stride,
            _padding: [0; 3],
        };
        unsafe {
            let bytes = std::slice::from_raw_parts(
                (&desc as *const GxoMatrixDescriptor) as *const u8,
                std::mem::size_of::<GxoMatrixDescriptor>(),
            );
            file.write_all(bytes).unwrap();
        }
        current_offset += m.mapped_soma_ids.len() as u32;
    }

    for m in matrices {
        let payload_bytes = unsafe {
            std::slice::from_raw_parts(
                m.mapped_soma_ids.as_ptr() as *const u8,
                m.mapped_soma_ids.len() * std::mem::size_of::<u32>(),
            )
        };
        file.write_all(payload_bytes)
            .expect("Failed to write soma IDs");
    }
}
