mod storage;
mod data_type;
mod column;

use column::{IColumn, ColumnVector};
use data_type::{IDataType};

fn run<T: IDataType + PartialEq + std::fmt::Debug>() -> std::io::Result<()> {
    let num_values: usize = 10_000;
    let mut col = ColumnVector::<T> {data: Vec::new()};

    for i in 0..num_values {
        let val = (i) as u64;
        let all_bytes = val.to_le_bytes();
        col.data.push(T::from_le_bytes(&all_bytes[..T::size_of()]));
    }
    println!("Generated {} values", col.len());

    let marks = storage::write_column(&col)?;
    println!("Wrote column.bin: {} granules", marks.len());

    storage::write_marks(&marks)?;
    println!("Wrote column.mrk");

    let mut row = 0usize;
    for (i, mark) in marks.iter().enumerate() {
        let granule: ColumnVector<T> = storage::read_granule(mark)?;                                        
        println!("  Read granule {}: {} values", i, granule.len());
        for (j, val) in granule.data.iter().enumerate() {
            assert_eq!(val, &col.data[row], "Mismatch at granule {} row {}", i, j);
            row += 1;                                                                                         
        }       
    }

    let target = 0usize;                                                                                    
    let granule: ColumnVector<T> = storage::read_granule(&marks[target])?;                                  
    println!("Direct read granule {}: first value = {:?}", target, granule.data);


    Ok(())
}

fn main() -> std::io::Result<()>{
    let args: Vec<String> = std::env::args().collect();
    let type_name = &args[1];

    match type_name.as_str() {
        "i8" => run::<i8>(),
        "i16" => run::<i16>(),
        "i32" => run::<i32>(),
        "i64" => run::<i64>(),                                                                                
        "u8"  => run::<u8>(),
        "u16" => run::<u16>(),                                                                                
        "u32" => run::<u32>(),
        "u64" => run::<u64>(),                                                                                
        "f32" => run::<f32>(),                                                                                
        "f64" => run::<f64>(),
        _     => panic!("Unknown type: {}", type_name), 
    }?;
    Ok(())
}
