mod mark;
mod storage;
mod data_type;
mod column;

use column::{IColumn};
use data_type::{IDataType};
use mark::{MarkReader};
use storage::{ColumnWriter, ColumnReader};

fn run<T: IDataType + PartialEq + std::fmt::Debug>() -> std::io::Result<()> {
    let num_values: usize = 10_000;
    let mut writer = ColumnWriter::create("column.bin", "column.mrk")?;

    for i in 0..num_values {
        let val = (i) as u64;
        let all_bytes = val.to_le_bytes();
        writer.push(T::from_le_bytes(&all_bytes[..T::size_of()]));
    }

    writer.flush()?;

    let marks = MarkReader::open("column.mrk")?.read_all()?;
    
    println!("Read {} marks from column.mrk", marks.len());

    let mut reader = ColumnReader::open("column.bin", "column.mrk")?;
    let granules: Vec<column::ColumnVector<T>> = reader.read_all()?;

    println!("Read {} granules", granules.len());
      
    let mut row = 0usize;
    for (i, granule) in granules.iter().enumerate() {
        println!("  Read granule {}: {} values", i, granule.len());
        for (j, val) in granule.data.iter().enumerate() {
            let expected = T::from_le_bytes(&(row as u64).to_le_bytes()[..T::size_of()]);
            assert_eq!(val, &expected, "Mismatch at granule {} row {}", i, j);
            row += 1;
        }
    }

    println!("Granule Data: {:?}", granules[1].data);


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
