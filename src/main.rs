// mod encoding;
// mod column_reader;
// mod column_writer;
// mod column_chunk;
// mod config;
// mod data_type;
// mod mark;
// mod schema;
// mod string_column_reader;
// mod string_column_writer;
// mod table_reader;
// mod table_writer;
// mod parser;
// mod aggregator;

// use std::path::PathBuf;

// use column_chunk::{ColumnChunk};
// use schema::{ColumnDef, DataType, TableDef};
// use table_reader::TableReader;
// use table_writer::TableWriter;

// fn main() -> std::io::Result<()>{
//     let sql = "INSERT INTO defaulttable (a, b) VALUES (1, 'hello'), (2, 'world'), (-3, 'z')";
//     let stmt = parser::parse(sql).unwrap();
//     println!("{stmt:#?}");
//     Ok(())
// }

mod aggregator;
mod column_chunk;
mod column_reader;
mod column_writer;
mod config;
mod data_type;
mod encoding;
mod mark;
mod parser;
mod schema;
mod string_column_reader;
mod string_column_writer;
mod table_reader;
mod table_writer;

use std::path::PathBuf;

use column_chunk::ColumnChunk;
use schema::{ColumnDef, DataType, TableDef};
use table_writer::TableWriter;

fn main() -> std::io::Result<()> {
    let table_dir: PathBuf = PathBuf::from("data").join("tinyolap_smoke");
    std::fs::create_dir_all(&table_dir)?;

    // Define and persist schema.
    let def = TableDef {
        name: "events".into(),
        columns: vec![
            ColumnDef { name: "ts".into(),  data_type: DataType::I64 },
            ColumnDef { name: "uid".into(), data_type: DataType::U32 },
            ColumnDef { name: "ok".into(),  data_type: DataType::Bool },
            ColumnDef { name: "tag".into(), data_type: DataType::Str },
        ],
        sort_key: vec![0],
    };
    TableDef::create(&table_dir, &def)?;

    // 2000 rows — spans multiple granules (GRANULE_SIZE = 512).
    let n = 2000usize;
    let ts:  Vec<i64>    = (0..n as i64).collect();
    let uid: Vec<u32>    = (0..n as u32).map(|x| x.wrapping_mul(7)).collect();
    let ok:  Vec<bool>   = (0..n).map(|i| i % 3 == 0).collect();
    let tag: Vec<String> = (0..n).map(|i| format!("row-{i}")).collect();

    let writer = TableWriter::open(table_dir)?;
    let meta = writer.insert(vec![
        ColumnChunk::I64(ts),
        ColumnChunk::U32(uid),
        ColumnChunk::Bool(ok),
        ColumnChunk::Str(tag),
    ])?;

    println!(
        "wrote part_{:05}: {} rows, {} columns, {} total compressed bytes",
        meta.part_id,
        meta.rows,
        meta.columns.len(),
        meta.columns.iter().map(|c| c.bin_bytes).sum::<u64>(),
    );

    Ok(())
}
