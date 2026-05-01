use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

use crate::config::{BLOCK_BUFFER_SIZE, GRANULE_SIZE};
use crate::column_writer::ColumnStats;
use crate::mark::{Mark, MarkWriter};

pub struct StringColumnWriter {
    data_file: File,
    mark_writer: MarkWriter,

    /// Uncompressed bytes for the current block (one or more whole granules).
    buffer: Vec<u8>,

    /// File position where the current `buffer`, once compressed, will land.
    /// Advances when we flush a block.
    block_offset: u64,

    /// Granule-in-progress state.
    granule_decompressed_offset: u64, // byte position in `buffer` where this granule began
    rows_in_current_granule: usize,
    bytes_in_current_granule: usize,

    total_rows: u64,
}

impl StringColumnWriter {
    pub fn create(part_dir: &Path, col_name: &str) -> io::Result<Self> {
        let data_path = part_dir.join(format!("{col_name}.bin"));
        let mark_path = part_dir.join(format!("{col_name}.mrk"));

        let data_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(data_path)?;

        Ok(Self {
            data_file,
            mark_writer: MarkWriter::create(&mark_path)?,
            buffer: Vec::with_capacity(BLOCK_BUFFER_SIZE * 2),
            block_offset: 0,
            granule_decompressed_offset: 0,
            rows_in_current_granule: 0,
            bytes_in_current_granule: 0,
            total_rows: 0,
        })
    }

    pub fn write_chunk(&mut self, values: &[String]) -> io::Result<()> {
        for s in values {
            let serialized_size = 4 + s.len();

            // Size cap: if adding this string would push the granule past 8KB,
            // seal the current granule first. (Never split a string.)
            if self.rows_in_current_granule > 0
                && self.bytes_in_current_granule + serialized_size > BLOCK_BUFFER_SIZE
            {
                self.seal_granule()?;
            }

            // Mark the start of a new granule (in buffer-relative bytes).
            if self.rows_in_current_granule == 0 {
                self.granule_decompressed_offset = self.buffer.len() as u64;
            }

            // Serialize: i32 length prefix + utf-8 bytes.
            self.buffer
                .extend_from_slice(&(s.len() as i32).to_le_bytes());
            self.buffer.extend_from_slice(s.as_bytes());

            self.rows_in_current_granule += 1;
            self.bytes_in_current_granule += serialized_size;
            self.total_rows += 1;

            // Count cap.
            if self.rows_in_current_granule == GRANULE_SIZE {
                self.seal_granule()?;
            }
        }
        Ok(())
    }

    /// Write the mark for the just-finished granule, then flush the block
    /// if it has reached the size threshold.
    fn seal_granule(&mut self) -> io::Result<()> {
        self.mark_writer.write(&Mark {
            block_offset: self.block_offset,
            decompressed_offset: self.granule_decompressed_offset,
        });

        self.rows_in_current_granule = 0;
        self.bytes_in_current_granule = 0;

        if self.buffer.len() >= BLOCK_BUFFER_SIZE {
            self.flush_block()?;
        }
        Ok(())
    }

    /// Compress `buffer`, write `[u32 compressed_len][compressed bytes]` to disk,
    /// advance `block_offset`, reset `buffer`.
    fn flush_block(&mut self) -> io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        let compressed = lz4_flex::compress_prepend_size(&self.buffer);
        self.data_file
            .write_all(&(compressed.len() as u32).to_le_bytes())?;
        self.data_file.write_all(&compressed)?;

        self.block_offset += 4 + compressed.len() as u64;
        self.buffer.clear();
        Ok(())
    }

    pub fn finish(mut self) -> io::Result<ColumnStats> {
        if self.rows_in_current_granule > 0 {
            self.seal_granule()?;
        }
        self.flush_block()?;

        let bin_bytes = self.data_file.metadata()?.len();
        self.mark_writer.flush()?;

        Ok(ColumnStats {
            rows: self.total_rows,
            bin_bytes,
        })
    }
}
