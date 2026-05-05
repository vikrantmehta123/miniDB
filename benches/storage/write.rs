use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use std::mem::size_of;
use tempfile::tempdir;
use tinyolap::storage::column_chunk::ColumnChunk;
use tinyolap::storage::schema::{ColumnDef, DataType, TableDef};
use tinyolap::storage::table_writer::TableWriter;

const ROWS: usize = 1_000_000;

fn single_col_schema() -> TableDef {
    TableDef {
        name: "bench".to_string(),
        columns: vec![ColumnDef { name: "ts".to_string(), data_type: DataType::I64 }],
        sort_key: vec![0],
    }
}

fn wide_numeric_schema() -> TableDef {
    TableDef {
        name: "bench".to_string(),
        columns: vec![
            ColumnDef { name: "ts".to_string(),    data_type: DataType::I64 },
            ColumnDef { name: "uid".to_string(),   data_type: DataType::U32 },
            ColumnDef { name: "val".to_string(),   data_type: DataType::F64 },
            ColumnDef { name: "flags".to_string(), data_type: DataType::U8  },
            ColumnDef { name: "score".to_string(), data_type: DataType::F32 },
        ],
        sort_key: vec![0],
    }
}

fn mixed_schema() -> TableDef {
    TableDef {
        name: "bench".to_string(),
        columns: vec![
            ColumnDef { name: "ts".to_string(),    data_type: DataType::I64 },
            ColumnDef { name: "uid".to_string(),   data_type: DataType::U32 },
            ColumnDef { name: "event".to_string(), data_type: DataType::Str },
            ColumnDef { name: "val".to_string(),   data_type: DataType::F64 },
        ],
        sort_key: vec![0],
    }
}

fn bench_single_i64(c: &mut Criterion) {
    let ts: Vec<i64> = (0..ROWS as i64).collect();
    let bytes = (ROWS * size_of::<i64>()) as u64;

    let mut group = c.benchmark_group("storage_write");
    group.throughput(Throughput::Bytes(bytes));
    group.bench_function("single_i64_1M", |b| {
        b.iter(|| {
            let dir = tempdir().unwrap();
            TableDef::create(dir.path(), &single_col_schema()).unwrap();
            let writer = TableWriter::open(dir.path().to_path_buf()).unwrap();
            writer.insert(vec![ColumnChunk::I64(black_box(ts.clone()))]).unwrap();
        });
    });
    group.finish();
}

fn bench_wide_numeric(c: &mut Criterion) {
    let ts: Vec<i64>    = (0..ROWS as i64).collect();
    let uid: Vec<u32>   = (0..ROWS as u32).map(|i| i % 100_000).collect();
    let val: Vec<f64>   = (0..ROWS).map(|i| i as f64 * 0.1).collect();
    let flags: Vec<u8>  = (0..ROWS).map(|i| (i % 256) as u8).collect();
    let score: Vec<f32> = (0..ROWS).map(|i| i as f32 * 0.01).collect();

    let bytes = (ROWS * (size_of::<i64>() + size_of::<u32>() + size_of::<f64>()
                       + size_of::<u8>()  + size_of::<f32>())) as u64;

    let mut group = c.benchmark_group("storage_write");
    group.throughput(Throughput::Bytes(bytes));
    group.bench_function("wide_numeric_5col_1M", |b| {
        b.iter(|| {
            let dir = tempdir().unwrap();
            TableDef::create(dir.path(), &wide_numeric_schema()).unwrap();
            let writer = TableWriter::open(dir.path().to_path_buf()).unwrap();
            writer.insert(vec![
                ColumnChunk::I64(black_box(ts.clone())),
                ColumnChunk::U32(black_box(uid.clone())),
                ColumnChunk::F64(black_box(val.clone())),
                ColumnChunk::U8(black_box(flags.clone())),
                ColumnChunk::F32(black_box(score.clone())),
            ]).unwrap();
        });
    });
    group.finish();
}

fn bench_mixed_with_strings(c: &mut Criterion) {
    const EVENT_NAMES: &[&str] = &[
        "click", "view", "purchase", "scroll", "hover",
        "login", "logout", "search", "share", "bookmark",
    ];

    let ts: Vec<i64>       = (0..ROWS as i64).collect();
    let uid: Vec<u32>      = (0..ROWS as u32).map(|i| i % 100_000).collect();
    let events: Vec<String> = (0..ROWS)
        .map(|i| EVENT_NAMES[i % EVENT_NAMES.len()].to_string())
        .collect();
    let val: Vec<f64>      = (0..ROWS).map(|i| i as f64 * 0.1).collect();

    let str_bytes: u64 = events.iter().map(|s| s.len() as u64).sum();
    let bytes = (ROWS * (size_of::<i64>() + size_of::<u32>() + size_of::<f64>())) as u64
              + str_bytes;

    let mut group = c.benchmark_group("storage_write");
    group.throughput(Throughput::Bytes(bytes));
    group.bench_function("mixed_4col_1M", |b| {
        b.iter(|| {
            let dir = tempdir().unwrap();
            TableDef::create(dir.path(), &mixed_schema()).unwrap();
            let writer = TableWriter::open(dir.path().to_path_buf()).unwrap();
            writer.insert(vec![
                ColumnChunk::I64(black_box(ts.clone())),
                ColumnChunk::U32(black_box(uid.clone())),
                ColumnChunk::Str(black_box(events.clone())),
                ColumnChunk::F64(black_box(val.clone())),
            ]).unwrap();
        });
    });
    group.finish();
}

criterion_group!(benches, bench_single_i64, bench_wide_numeric, bench_mixed_with_strings);
criterion_main!(benches);
