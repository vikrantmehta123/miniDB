use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

pub const GRANULE_SIZE: usize = 512;
pub const BLOCK_BUFFER_SIZE: usize = 8 * 1024; // size of uncompressed buffer before compression
                                           // happens


pub struct Mark {
    pub block_offset: u64,
    pub granule_offset: u64,
    pub num_rows: u64,
}

pub fn write_column(values: &[i64]) -> std::io::Result<Vec<Mark>> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("column.bin")?;

    let mut buffer: Vec<u8> = Vec::with_capacity(BLOCK_BUFFER_SIZE);
    let mut block_offset: u64 = 0;

    let mut marks: Vec<Mark> =  Vec::new();
    let mut rows_in_buffer: usize = 0; // The number of i64 values in the buffer currently 

    for &v in values {

        // Emit a mark at the start of the granule
        if rows_in_buffer % GRANULE_SIZE == 0 {
            marks.push(Mark {
                block_offset, 
                granule_offset: (rows_in_buffer * 8) as u64,
                num_rows: GRANULE_SIZE as u64 // Fixed number of rows in a granule
            });
        }   

        buffer.extend_from_slice(&v.to_le_bytes());
        rows_in_buffer += 1;

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
    
    // Last granule can have < 512 rows. If so, we need to fix the num_rows 
    let total = values.len();
    let remainder = total % GRANULE_SIZE;
    if remainder != 0 {
        if let Some(last) = marks.last_mut() {
            last.num_rows = remainder as u64;
        }
    }


    Ok(marks)
}

pub fn write_marks(marks: &[Mark]) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("column.mrk")?;

    for mark in marks {
        file.write_all(&mark.block_offset.to_le_bytes())?;
        file.write_all(&mark.granule_offset.to_le_bytes())?;
        file.write_all(&mark.num_rows.to_le_bytes())?;
    }

    Ok(())
}


pub fn read_granule(mark: &Mark) -> std::io::Result<Vec<i64>> {
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
    let end = start + mark.num_rows as usize * 8;

    let values = decompressed[start..end]
        .chunks_exact(8)
        .map(|b| i64::from_le_bytes(b.try_into().unwrap()))
        .collect();

    Ok(values)
}
