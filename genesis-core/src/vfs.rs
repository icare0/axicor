use std::collections::HashMap;
use memmap2::Mmap;
use std::fs::File;
use std::path::Path;

pub struct AxicArchive {
    pub mmap: Mmap,
    pub toc: HashMap<String, (usize, usize)>,
}

unsafe impl Send for AxicArchive {}
unsafe impl Sync for AxicArchive {}

impl AxicArchive {
    pub fn open(path: &Path) -> Option<Self> {
        let file = File::open(path).ok()?;
        let mmap = unsafe { Mmap::map(&file).ok()? };
        if mmap.len() < 12 || &mmap[0..4] != b"AXIC" { return None; }
        
        let count = u32::from_le_bytes(mmap[8..12].try_into().unwrap()) as usize;
        let mut toc = HashMap::new();
        let mut offset = 12;
        
        for _ in 0..count {
            if offset + 272 > mmap.len() { break; }
            let toc_entry = &mmap[offset .. offset + 272];
            let path_len = toc_entry[0..256].iter().position(|&c| c == 0).unwrap_or(256);
            if let Ok(s) = std::str::from_utf8(&toc_entry[0..path_len]) {
                let f_offset = u64::from_le_bytes(toc_entry[256..264].try_into().unwrap()) as usize;
                let f_size = u64::from_le_bytes(toc_entry[264..272].try_into().unwrap()) as usize;
                toc.insert(s.to_string(), (f_offset, f_size));
            }
            offset += 272;
        }
        Some(Self { mmap, toc })
    }

    pub fn get_file(&self, path: &str) -> Option<&[u8]> {
        let (off, size) = self.toc.get(path)?;
        Some(&self.mmap[*off .. *off + *size])
    }
}
