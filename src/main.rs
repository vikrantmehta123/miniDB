mod column;
use column::Column;
use std::path::Path;

fn main() -> std::io::Result<()> {
    let path = Path::new("col_i64.bin");
    let mut col = Column::open(path, 0);
    
    let data: Vec<i64> = (0..1500).map(|i| i as i64 * 10).collect();

    col.append_chunk(&data)?;
    
    println!("Wrote {} values", col.num_rows);

    match col.read_chunk(0)? {
        Some(chunk) =>  println!("Chunk 0: {} values, first={}, last={}", chunk.len(), chunk[0],
  chunk[chunk.len()-1]),
          None => println!("Chunk 0 not found"),
    }

    match col.read_chunk(1)? {
          Some(chunk) => println!("Chunk 1: {} values, first={}, last={}", chunk.len(), chunk[0],
  chunk[chunk.len()-1]),
          None => println!("Chunk 1 not found"),
      }


    Ok(())
}
