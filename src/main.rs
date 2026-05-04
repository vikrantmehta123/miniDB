mod aggregator;
mod config;
mod data_type;
mod encoding;
mod storage;


use std::path::PathBuf;

use storage::column_chunk::ColumnChunk;
use storage::schema::{ColumnDef, DataType, TableDef};
use storage::table_reader::TableReader;
use storage::table_writer::TableWriter;

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
    let n = 200usize;
    let ts:  Vec<i64>    = (0..n as i64).collect();
    let uid: Vec<u32>    = (0..n as u32).map(|x| x.wrapping_mul(7)).collect();
    let ok:  Vec<bool>   = (0..n).map(|i| i % 3 == 0).collect();
    let tag: Vec<String> = (0..n).map(|i| format!("row-{i}")).collect();

    // Write.
    let writer = TableWriter::open(table_dir.clone())?;
    let meta = writer.insert(vec![
        ColumnChunk::I64(ts.clone()),
        ColumnChunk::U32(uid.clone()),
        ColumnChunk::Bool(ok.clone()),
        ColumnChunk::Str(tag.clone()),
    ])?;
    println!(
        "wrote part_{:05}: {} rows, {} columns, {} total compressed bytes",
        meta.part_id,
        meta.rows,
        meta.columns.len(),
        meta.columns.iter().map(|c| c.bin_bytes).sum::<u64>(),
    );

    // Read back.
    let mut reader = TableReader::open(&table_dir, meta.part_id)?;

    let mut ts_back:  Vec<i64>    = Vec::new();
    let mut uid_back: Vec<u32>    = Vec::new();
    let mut ok_back:  Vec<bool>   = Vec::new();
    let mut tag_back: Vec<String> = Vec::new();

    for g in 0..reader.granule_count() {
        let chunks = reader.read_granule(g)?;
        for chunk in chunks {
            match chunk {
                ColumnChunk::I64(v)  => ts_back.extend(v),
                ColumnChunk::U32(v)  => uid_back.extend(v),
                ColumnChunk::Bool(v) => ok_back.extend(v),
                ColumnChunk::Str(v)  => tag_back.extend(v),
                _ => {}
            }
        }
    }

    // Verify round-trip.
    assert_eq!(ts,  ts_back,  "ts mismatch");
    assert_eq!(uid, uid_back, "uid mismatch");
    assert_eq!(ok,  ok_back,  "ok mismatch");
    assert_eq!(tag, tag_back, "tag mismatch");

    println!("round-trip OK ({} rows × 4 cols)", ts_back.len());
    println!("first ts:  {:?}", &ts_back[..5]);
    println!("first uid: {:?}", &uid_back[..5]);
    println!("first ok:  {:?}", &ok_back[..5]);
    println!("first tag: {:?}", &tag_back[..5]);

    Ok(())
}
