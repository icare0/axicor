// genesis-baker/src/bake/output_map.rs
use genesis_core::constants::GXO_MAGIC;
use genesis_core::config::IoConfig;
use genesis_core::hash::fnv1a_32;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

/// Запекает .gxo файл используя Z-Sort алгоритм для выбора сом-кандидатов.
pub fn bake_outputs(
    out_dir: &Path,
    io_config: &IoConfig,
    zone_width_um: f32,
    zone_depth_um: f32,
    neurons_packed_pos: &[u32], // Массив PackedPosition, где индекс = Dense_ID сомы
) {
    if io_config.outputs.is_empty() {
        return;
    }

    let mut total_pixels = 0;
    for m in &io_config.outputs {
        total_pixels += m.width * m.height;
    }

    let mut payload_soma_ids = vec![0u32; total_pixels as usize];
    let mut current_offset = 0;

    for matrix in &io_config.outputs {
        let pixels = matrix.width * matrix.height;
        
        // Z-Sort: Для каждого пикселя ищем сому с минимальным Z
        for py in 0..matrix.height {
            for px in 0..matrix.width {
                let x_min = (px as f32 / matrix.width as f32) * zone_width_um;
                let x_max = ((px + 1) as f32 / matrix.width as f32) * zone_width_um;
                let y_min = (py as f32 / matrix.height as f32) * zone_depth_um;
                let y_max = ((py + 1) as f32 / matrix.height as f32) * zone_depth_um;

                let mut best_soma_id = u32::MAX;
                let mut min_z = u32::MAX;

                for (dense_id, &packed) in neurons_packed_pos.iter().enumerate() {
                    let vx = (packed & 0x3FF) as f32; 
                    let vy = ((packed >> 10) & 0x3FF) as f32; 
                    let vz = (packed >> 20) & 0xFF;

                    if vx >= x_min && vx < x_max && vy >= y_min && vy < y_max {
                        if vz < min_z {
                            min_z = vz;
                            best_soma_id = dense_id as u32;
                        }
                    }
                }

                if best_soma_id == u32::MAX {
                    // Fail gracefully if density is too low
                    best_soma_id = 0; 
                }

                let pixel_idx = (py * matrix.width) + px;
                payload_soma_ids[(current_offset + pixel_idx) as usize] = best_soma_id;
            }
        }
        current_offset += pixels;
    }

    write_gxo_binary(out_dir, io_config, total_pixels, &payload_soma_ids);
}

fn write_gxo_binary(
    out_dir: &Path, 
    io_config: &IoConfig, 
    total_pixels: u32, 
    payload: &[u32]
) {
    let path = out_dir.join("shard.gxo");
    let mut file = BufWriter::new(File::create(path).expect("Failed to create .gxo file"));

    // Header (12 bytes)
    file.write_all(&GXO_MAGIC.to_le_bytes()).unwrap();
    file.write_all(&[1u8, 0u8]).unwrap(); // Version 1, Padding 1
    file.write_all(&(io_config.outputs.len() as u16).to_le_bytes()).unwrap();
    file.write_all(&total_pixels.to_le_bytes()).unwrap();

    let mut current_offset: u32 = 0;
    for m in &io_config.outputs {
        let name_hash = fnv1a_32(m.name.as_bytes());
        file.write_all(&name_hash.to_le_bytes()).unwrap();
        file.write_all(&current_offset.to_le_bytes()).unwrap();
        file.write_all(&(m.width as u16).to_le_bytes()).unwrap();
        file.write_all(&(m.height as u16).to_le_bytes()).unwrap();
        file.write_all(&(m.stride as u8).to_le_bytes()).unwrap();
        file.write_all(&[0, 0, 0]).unwrap(); // Padding (3 bytes)
        
        current_offset += m.width * m.height;
    }

    let payload_bytes = unsafe {
        std::slice::from_raw_parts(
            payload.as_ptr() as *const u8,
            payload.len() * 4
        )
    };
    file.write_all(payload_bytes).unwrap();
}
