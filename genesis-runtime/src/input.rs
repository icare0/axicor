use std::fs::File;
use std::io::Read;
use std::path::Path;
use anyhow::{Context, Result};

pub struct GxiMapDescriptor {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub axon_offset: u32,
}

pub struct GxiFile {
    pub magic: u32,
    pub version: u16,
    pub maps: Vec<GxiMapDescriptor>,
    pub axon_ids: Vec<u32>,
}

impl GxiFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let mut file = File::open(path.as_ref())
            .with_context(|| format!("Failed to open .gxi file at {:?}", path.as_ref()))?;
            
        let mut magic_buf = [0u8; 4];
        file.read_exact(&mut magic_buf)?;
        let magic = u32::from_le_bytes(magic_buf);
        if magic != 0x47584930 {
            anyhow::bail!("Invalid GXI magic");
        }

        let mut version_buf = [0u8; 2];
        file.read_exact(&mut version_buf)?;
        let version = u16::from_le_bytes(version_buf);

        let mut num_maps_buf = [0u8; 2];
        file.read_exact(&mut num_maps_buf)?;
        let num_maps = u16::from_le_bytes(num_maps_buf);

        let mut maps = Vec::with_capacity(num_maps as usize);
        for _ in 0..num_maps {
            let mut name_len_buf = [0u8; 2];
            file.read_exact(&mut name_len_buf)?;
            let name_len = u16::from_le_bytes(name_len_buf);

            let mut name_buf = vec![0u8; name_len as usize];
            file.read_exact(&mut name_buf)?;
            let name = String::from_utf8(name_buf)?;

            let mut w_buf = [0u8; 4];
            file.read_exact(&mut w_buf)?;
            let width = u32::from_le_bytes(w_buf);

            let mut h_buf = [0u8; 4];
            file.read_exact(&mut h_buf)?;
            let height = u32::from_le_bytes(h_buf);

            let mut o_buf = [0u8; 4];
            file.read_exact(&mut o_buf)?;
            let axon_offset = u32::from_le_bytes(o_buf);

            maps.push(GxiMapDescriptor {
                name,
                width,
                height,
                axon_offset,
            });
        }

        let mut rest = Vec::new();
        file.read_to_end(&mut rest)?;

        let mut axon_ids = Vec::with_capacity(rest.len() / 4);
        for chunk in rest.chunks_exact(4) {
            let arr: [u8; 4] = chunk.try_into().unwrap();
            axon_ids.push(u32::from_le_bytes(arr));
        }

        Ok(Self {
            magic,
            version,
            maps,
            axon_ids,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::fs;
    use std::env;

    #[test]
    fn test_gxi_parse_valid() {
        let path = env::temp_dir().join(format!("test_{}.gxi", std::process::id()));
        let mut file = fs::File::create(&path).unwrap();
        // Header
        file.write_all(&0x47584930u32.to_le_bytes()).unwrap(); // Magic
        file.write_all(&1u16.to_le_bytes()).unwrap(); // Version
        file.write_all(&1u16.to_le_bytes()).unwrap(); // Num maps
        
        // Map 1
        let name = "retina".as_bytes();
        file.write_all(&(name.len() as u16).to_le_bytes()).unwrap();
        file.write_all(name).unwrap();
        file.write_all(&10u32.to_le_bytes()).unwrap(); // W
        file.write_all(&10u32.to_le_bytes()).unwrap(); // H
        file.write_all(&0u32.to_le_bytes()).unwrap(); // Offset

        // Body (axon ids)
        for i in 0..100 {
            file.write_all(&(i as u32).to_le_bytes()).unwrap();
        }
        file.flush().unwrap();

        let parsed = GxiFile::load(&path).unwrap();
        assert_eq!(parsed.magic, 0x47584930);
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.maps.len(), 1);
        assert_eq!(parsed.maps[0].name, "retina");
        assert_eq!(parsed.maps[0].width, 10);
        assert_eq!(parsed.maps[0].height, 10);
        assert_eq!(parsed.maps[0].axon_offset, 0);
        assert_eq!(parsed.axon_ids.len(), 100);
        for i in 0..100 {
            assert_eq!(parsed.axon_ids[i], i as u32);
        }
        
        fs::remove_file(path).ok();
    }
}
