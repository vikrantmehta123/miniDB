mod storage;

fn main() -> std::io::Result<()> {
    let data: Vec<i64> = (0..10_000).map(|i| i as i64).collect();
    let marks = storage::write_column(&data)?;
    
    println!("Total marks: {}", marks.len()); 

   for (i, m) in marks.iter().enumerate() {
       println!("Mark {:>2}  block_offset={:>6}  granule_offset={:>5}  num_rows={}",
          i, m.block_offset, m.granule_offset, m.num_rows);
   }

    storage::write_marks(&marks)?;
    println!("Wrote {} marks to column.mrk", marks.len());

    let granule = storage::read_granule(&marks[2])?;
    println!("Third granule: {} values, first={}, last={}",
      granule.len(), granule[0], granule[granule.len()-1]);

    println!("First 5 values: {:?}", &granule[..5]);

    Ok(())
}
