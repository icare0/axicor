            if let Some(io_bytes) = archive.get_file(&io_vfs_path) {
                let io_str = std::str::from_utf8(io_bytes)?;
                if let Ok(io_config) = toml::from_str::<genesis_core::config::io::IoConfig>(io_str) {
                    expected_inputs = !io_config.input.is_empty();
                    expected_outputs = !io_config.output.is_empty();

                    let mut current_bit_offset = 0u32;
                    for matrix in &io_config.input {
                        for pin in &matrix.pin {
                            let hash = genesis_core::hash::fnv1a_32(pin.name.as_bytes());
                            matrix_offsets.insert(hash, (current_bit_offset / 8) as u32);
                            current_bit_offset += pin.width * pin.height;
                            current_bit_offset = (current_bit_offset + 31) & !31;
                        }
                    }

                    let mut current_pixel_offset = 0usize;
                    for matrix in &io_config.output {
                        for pin in &matrix.pin {
                            let hash = genesis_core::hash::fnv1a_32(pin.name.as_bytes());
                            let chunk_pixels = (pin.width * pin.height) as usize;

                            let target = zone_manifest.network.external_udp_out_target
                                .clone()
                                .unwrap_or_else(|| "127.0.0.1:8092".to_string());

                            output_routes.entry(zone_hash).or_insert_with(Vec::new)
                                .push((target.clone(), hash, current_pixel_offset, chunk_pixels));
                            println!("[Boot] Registered Output Route: {} (0x{:08X}) -> {}", pin.name, hash, target);
                            
                            current_pixel_offset += chunk_pixels;
                        }
                    }
                }
            }
