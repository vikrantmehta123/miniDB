use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

use crate::data_type::IDataType;
use crate::mark::{Mark, MarkReader};

pub struct ColumnReader {
    bin: File,
    marks: Vec<Mark>,
    /// (block_offset, decompressed bytes) — single-block cache.
    cache: Option<(u64, Vec<u8>)>,
}

impl ColumnReader {
    pub fn open(part_dir: &Path, col_name: &str) -> io::Result<Self> {
        let bin_path = part_dir.join(format!("{col_name}.bin"));
        let mrk_path = part_dir.join(format!("{col_name}.mrk"));

        let bin = File::open(bin_path)?;
        let marks = MarkReader::open(&mrk_path)?.read_all()?;

        Ok(Self { bin, marks, cache: None })
    }

    pub fn granule_count(&self) -> usize {
        self.marks.len()
    }

    pub fn read_granule<T: IDataType>(&mut self, idx: usize) -> io::Result<Vec<T>> {
        let mark = &self.marks[idx];

        // Make sure the right block is in the cache.
        let cache_hit = matches!(&self.cache, Some((off, _)) if *off == mark.block_offset);
        if !cache_hit {
            self.bin.seek(SeekFrom::Start(mark.block_offset))?;

            let mut len_buf = [0u8; 4];
            self.bin.read_exact(&mut len_buf)?;
            let compressed_len = u32::from_le_bytes(len_buf) as usize;

            let mut compressed = vec![0u8; compressed_len];
            self.bin.read_exact(&mut compressed)?;

            let bytes = lz4_flex::decompress_size_prepended(&compressed)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            self.cache = Some((mark.block_offset, bytes));
        }

        let block = &self.cache.as_ref().unwrap().1;

        // Granule's byte range inside the decompressed block:
        //   start = this mark's decompressed_offset
        //   end   = next mark's decompressed_offset, if it's in the *same* block;
        //           otherwise the block ends here.
        let start = mark.decompressed_offset as usize;
        let end = match self.marks.get(idx + 1) {
            Some(next) if next.block_offset == mark.block_offset => {
                next.decompressed_offset as usize
            }
            _ => block.len(),
        };

        let elem_size = std::mem::size_of::<T>();
        debug_assert!(
            (end - start) % elem_size == 0,
            "granule byte range not a multiple of element size"
        );
        let count = (end - start) / elem_size;

        let mut out = Vec::with_capacity(count);
        let mut cursor = start;
        for _ in 0..count {
            let v = T::from_le_bytes(&block[cursor..cursor + elem_size]);
            out.push(v);
            cursor += elem_size;
        }
        Ok(out)
    }

    /// Convenience: read every granule in order.
    pub fn read_all<T: IDataType>(&mut self) -> io::Result<Vec<T>> {
        let mut out = Vec::new();
        for i in 0..self.marks.len() {
            out.extend(self.read_granule::<T>(i)?);
        }
        Ok(out)
    }
}
