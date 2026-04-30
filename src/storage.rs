use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

use crate::data_type::IDataType;
use crate::column::{ColumnVector, IColumn};
use crate::mark::{Mark, MarkWriter};

pub const GRANULE_SIZE: usize = 512;
pub const BLOCK_BUFFER_SIZE: usize = 8 * 1024; // size of uncompressed buffer before compression
                                           // happens

pub fn write_column<T: IDataType>(col: &ColumnVector<T>) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("column.bin")?;

    let mut mark_writer = MarkWriter::create("column.mrk")?;

    let mut buffer: Vec<u8> = Vec::with_capacity(BLOCK_BUFFER_SIZE);
    let mut block_offset: u64 = 0;
    let mut rows_in_buffer: usize = 0; // The number of i64 values in the buffer currently 

    let total = col.len();
    let mut granule_start = 0;

    while granule_start < total {
        let limit = GRANULE_SIZE.min(total - granule_start);
        
        mark_writer.write(&Mark {
            block_offset, 
            granule_offset: (rows_in_buffer * T::size_of()) as u64,
            num_rows: limit as u64 // Fixed number of rows in a granule
        });
        
        col.serialize_binary_bulk(&mut buffer, granule_start, limit);
        rows_in_buffer += limit;
        granule_start += limit;

        if buffer.len() >= BLOCK_BUFFER_SIZE {

            let compressed = lz4_flex::compress_prepend_size(&buffer);
            
            file.write_all(&(compressed.len() as u32).to_le_bytes())?;  // length header
            file.write_all(&compressed)?;

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
    
        file.write_all(&(compressed.len() as u32).to_le_bytes())?;  // length header
        file.write_all(&compressed)?;

        println!("Flushed final block at offset {}: {} bytes -> {} bytes compressed",
              block_offset, buffer.len(), compressed.len());

    }
    mark_writer.flush()?;
    Ok(())
}

pub fn read_granule<T: IDataType>(mark: &Mark) -> std::io::Result<ColumnVector<T>> {
    let mut file = File::open("column.bin")?;
    file.seek(SeekFrom::Start(mark.block_offset))?;

    let mut len_buf = [0u8; 4];

    file.read_exact(&mut len_buf)?;

    let compressed_len = u32::from_le_bytes(len_buf) as usize;

    let mut compressed = vec![0u8; compressed_len];

    file.read_exact(&mut compressed)?;

    let decompressed = lz4_flex::decompress_size_prepended(&compressed)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?; 

    let start = mark.granule_offset as usize;
    let end = start + mark.num_rows as usize * T::size_of();
    
    let granule_bytes = &decompressed[start..end];

    let mut col: ColumnVector<T> = ColumnVector { data: Vec::new() };
    col.deserialize_binary_bulk(granule_bytes);

    Ok(col)
}
