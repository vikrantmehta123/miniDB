mod mark;
mod storage;
mod data_type;
mod column;
mod string_column;
mod config;
mod schema;
mod part;

use std::path::{Path};
use column::{IColumn};
use data_type::{IDataType};
use mark::{MarkReader};
use storage::{ColumnWriter, ColumnReader};
use string_column::{StringColumnWriter, StringColumnReader};
use schema::{TableDef, ColumnDef, DataType};
use part::{Part};


fn run_part() -> std::io::Result<()> {
    let table_dir = Path::new("data/events");
    let def = TableDef {
        name: "events".to_string(),
        columns: vec![
            ColumnDef { name: "timestamp".to_string(), data_type: DataType::I64 },
            ColumnDef { name: "user_id".to_string(),   data_type: DataType::U32 },
            ColumnDef { name: "label".to_string(),     data_type: DataType::Str },
        ],
        sort_key: vec![0],
    };

    TableDef::create(table_dir, &def)?;

    let part_dir = TableDef::part_dir(table_dir, 1);
    let part = Part::new(part_dir);
    part.create_dir()?;

    for col in &def.columns {
        println!("{}", part.column_bin_path(col).display());
        println!("{}", part.column_mrk_path(col).display());
    }

    Ok(())
}


fn run_string() -> std::io::Result<()> {
    let num_values = 2000;
    let mut writer = StringColumnWriter::create("string_column.bin", "string_column.mrk")?;

    let mut expected: Vec<String> = Vec::new();
    for i in 0..num_values {
        let s = format!("row_{}", i);
        expected.push(s.clone());
        writer.push(s);
    }
    writer.flush()?;

    let mut reader = StringColumnReader::open("string_column.bin", "string_column.mrk")?;
    let granules = reader.read_all()?;

    println!("Read {} granules", granules.len());
    println!("Granule Data: {:?}", granules[1].data);

    let mut row = 0usize;
    for (i, granule) in granules.iter().enumerate() {
        println!("  Granule {}: {} strings", i, granule.data.len());
        for (j, val) in granule.data.iter().enumerate() {
            assert_eq!(val, &expected[row], "Mismatch at granule {} row {}", i, j);
            row += 1;
        }
    }

    println!("Round-trip OK: {} strings verified", row);
    Ok(())
}



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
        "bool" => run::<bool>(),
        "string" => run_string(),
        "part" => run_part(),
        _     => panic!("Unknown type: {}", type_name), 
    }?;
    Ok(())
}
