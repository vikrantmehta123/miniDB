use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path};

use crate::data_type::IDataType;
use crate::column::{ColumnVector, IColumn};
use crate::mark::{Mark, MarkReader, MarkWriter};

use crate::config::{GRANULE_SIZE, BLOCK_BUFFER_SIZE};


pub struct ColumnWriter<T: IDataType> {
    pub col: ColumnVector<T>,
    mark_writer: MarkWriter,
    data_file: File,
}

impl<T: IDataType> ColumnWriter<T> {
    pub fn create(data_path: &Path, mark_path: &Path) -> std::io::Result<Self>{
        let data_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(data_path)?;

        let mark_writer = MarkWriter::create(mark_path)?;
        Ok(
            ColumnWriter { 
                col: ColumnVector { data: Vec::new()
            },
            mark_writer, 
            data_file,
        })
    }

    pub fn push(&mut self, val: T){
        self.col.data.push(val);
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        let mut buffer: Vec<u8> = Vec::with_capacity(BLOCK_BUFFER_SIZE);
        let mut block_offset: u64 = 0;
        let mut rows_in_buffer: usize = 0; // The number of i64 values in the buffer currently 
        let total = self.col.len();
        let mut granule_start = 0;

        while granule_start < total {
            let limit = GRANULE_SIZE.min(total - granule_start);
            
            self.mark_writer.write(&Mark {
                block_offset, 
                granule_offset: (rows_in_buffer * T::size_of()) as u64,
                num_rows: limit as u64 // Fixed number of rows in a granule
            });
            
            self.col.serialize_binary_bulk(&mut buffer, granule_start, limit);
            rows_in_buffer += limit;
            granule_start += limit;

            if buffer.len() >= BLOCK_BUFFER_SIZE {

                let compressed = lz4_flex::compress_prepend_size(&buffer);
                
                self.data_file.write_all(&(compressed.len() as u32).to_le_bytes())?;  // length header
                self.data_file.write_all(&compressed)?;

                println!("Flushed block at offset {}: {} bytes -> {} bytes compressed",  
                    block_offset, buffer.len(), compressed.len());

                block_offset += 4 + compressed.len() as u64;  // +4 for the header
                buffer.clear();
                rows_in_buffer = 0;
            }
        }
            
        // Flush the final partial block
        if !buffer.is_empty() {
            let compressed = lz4_flex::compress_prepend_size(&buffer);
        
            self.data_file.write_all(&(compressed.len() as u32).to_le_bytes())?;  // length header
            self.data_file.write_all(&compressed)?;

            println!("Flushed final block at offset {}: {} bytes -> {} bytes compressed",
                block_offset, buffer.len(), compressed.len());

        }
        self.mark_writer.flush()?;

        Ok(())
    }
}


pub struct ColumnReader {
    mark_reader: MarkReader, 
    data_file: File, 
    block_cache: Option<(u64, Vec<u8>)>, // The offset in the data_file where the compressed block starts and the vector for it.
}

impl ColumnReader {
    pub fn open(data_path: &Path, mark_path: &Path) -> std::io::Result<Self>{
        Ok(ColumnReader { 
            data_file: File::open(data_path)?, 
            mark_reader: MarkReader::open(mark_path)?, 
            block_cache: None,
        })
    }

    pub fn read_granule<T: IDataType>(&mut self, mark: &Mark) -> std::io::Result<ColumnVector<T>>{
        let cache_hit = matches!(&self.block_cache, Some((offset, _)) if *offset == mark.block_offset);

        if !cache_hit {
            self.data_file.seek(SeekFrom::Start(mark.block_offset))?;
            let mut len_buf = [0u8; 4];
            self.data_file.read_exact(&mut len_buf)?;
            let compressed_len = u32::from_le_bytes(len_buf) as usize;
            let mut compressed = vec![0u8; compressed_len];
            
            self.data_file.read_exact(&mut compressed)?;

            let bytes = lz4_flex::decompress_size_prepended(&compressed)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?; 

           self.block_cache = Some((mark.block_offset, bytes));
        }

        let decompressed = &self.block_cache.as_ref().unwrap().1;

        let start = mark.granule_offset as usize;
        let end = start + mark.num_rows as usize * T::size_of();
        let mut col: ColumnVector<T> = ColumnVector { data: Vec::new() };

        col.deserialize_binary_bulk(&decompressed[start..end]);
        Ok(col)
    }

    pub fn read_all<T: IDataType>(&mut self) -> std::io::Result<Vec<ColumnVector<T>>> {
        let marks = self.mark_reader.read_all()?;
        marks.iter().map(|mark| self.read_granule(mark)).collect()
    }

}