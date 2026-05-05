use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use std::mem::size_of;
use tempfile::tempdir;
use tinyolap::processors::full_scan::FullScan;
use tinyolap::processors::processor::Processor;
use tinyolap::storage::column_chunk::ColumnChunk;
use tinyolap::storage::schema::{ColumnDef, DataType, TableDef};
use tinyolap::storage::table_writer::TableWriter;

const ROWS: usize = 1_000_000;

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

fn write_wide_numeric(dir: &std::path::Path) {
    TableDef::create(dir, &wide_numeric_schema()).unwrap();
    let writer = TableWriter::open(dir.to_path_buf()).unwrap();
    let ts: Vec<i64>    = (0..ROWS as i64).collect();
    let uid: Vec<u32>   = (0..ROWS as u32).map(|i| i % 100_000).collect();
    let val: Vec<f64>   = (0..ROWS).map(|i| i as f64 * 0.1).collect();
    let flags: Vec<u8>  = (0..ROWS).map(|i| (i % 256) as u8).collect();
    let score: Vec<f32> = (0..ROWS).map(|i| i as f32 * 0.01).collect();
    writer.insert(vec![
        ColumnChunk::I64(ts),
        ColumnChunk::U32(uid),
        ColumnChunk::F64(val),
        ColumnChunk::U8(flags),
        ColumnChunk::F32(score),
    ]).unwrap();
}

fn bench_single_col_pruned(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    write_wide_numeric(dir.path());

    let col_def = vec![ColumnDef { name: "ts".to_string(), data_type: DataType::I64 }];
    let bytes = (ROWS * size_of::<i64>()) as u64;

    let mut group = c.benchmark_group("storage_scan");
    group.throughput(Throughput::Bytes(bytes));
    group.bench_function("single_col_pruned_1M", |b| {
        b.iter(|| {
            let mut scan = FullScan::new(dir.path().to_path_buf(), col_def.clone()).unwrap();
            while let Some(batch) = scan.next_batch() {
                black_box(batch.unwrap());
            }
        });
    });
    group.finish();
}

fn bench_all_cols(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    write_wide_numeric(dir.path());

    let col_defs = wide_numeric_schema().columns;
    let bytes = (ROWS * (size_of::<i64>() + size_of::<u32>() + size_of::<f64>()
                       + size_of::<u8>()  + size_of::<f32>())) as u64;

    let mut group = c.benchmark_group("storage_scan");
    group.throughput(Throughput::Bytes(bytes));
    group.bench_function("all_cols_5col_1M", |b| {
        b.iter(|| {
            let mut scan = FullScan::new(dir.path().to_path_buf(), col_defs.clone()).unwrap();
            while let Some(batch) = scan.next_batch() {
                black_box(batch.unwrap());
            }
        });
    });
    group.finish();
}

fn bench_string_col(c: &mut Criterion) {
    const EVENT_NAMES: &[&str] = &[
        "click", "view", "purchase", "scroll", "hover",
        "login", "logout", "search", "share", "bookmark",
    ];

    let dir = tempdir().unwrap();
    TableDef::create(dir.path(), &mixed_schema()).unwrap();
    let writer = TableWriter::open(dir.path().to_path_buf()).unwrap();
    let ts: Vec<i64>        = (0..ROWS as i64).collect();
    let uid: Vec<u32>       = (0..ROWS as u32).map(|i| i % 100_000).collect();
    let events: Vec<String> = (0..ROWS)
        .map(|i| EVENT_NAMES[i % EVENT_NAMES.len()].to_string())
        .collect();
    let val: Vec<f64>       = (0..ROWS).map(|i| i as f64 * 0.1).collect();
    let str_bytes: u64      = events.iter().map(|s| s.len() as u64).sum();
    writer.insert(vec![
        ColumnChunk::I64(ts),
        ColumnChunk::U32(uid),
        ColumnChunk::Str(events),
        ColumnChunk::F64(val),
    ]).unwrap();

    let col_def = vec![ColumnDef { name: "event".to_string(), data_type: DataType::Str }];

    let mut group = c.benchmark_group("storage_scan");
    group.throughput(Throughput::Bytes(str_bytes));
    group.bench_function("string_col_1M", |b| {
        b.iter(|| {
            let mut scan = FullScan::new(dir.path().to_path_buf(), col_def.clone()).unwrap();
            while let Some(batch) = scan.next_batch() {
                black_box(batch.unwrap());
            }
        });
    });
    group.finish();
}

fn bench_multipart_scan(c: &mut Criterion) {
    const CONFIGS: &[(usize, usize)] = &[
        (10,  100_000),
        (100,  10_000),
    ];

    let schema = TableDef {
        name: "bench".to_string(),
        columns: vec![ColumnDef { name: "ts".to_string(), data_type: DataType::I64 }],
        sort_key: vec![0],
    };
    let col_def = vec![ColumnDef { name: "ts".to_string(), data_type: DataType::I64 }];

    let mut group = c.benchmark_group("storage_scan");

    for &(parts, rows_per_part) in CONFIGS {
        let total_rows = parts * rows_per_part;
        let bytes = (total_rows * size_of::<i64>()) as u64;

        let dir = tempdir().unwrap();
        TableDef::create(dir.path(), &schema).unwrap();
        for i in 0..parts {
            let start = (i * rows_per_part) as i64;
            let data: Vec<i64> = (start..start + rows_per_part as i64).collect();
            let writer = TableWriter::open(dir.path().to_path_buf()).unwrap();
            writer.insert(vec![ColumnChunk::I64(data)]).unwrap();
        }

        group.throughput(Throughput::Bytes(bytes));
        group.bench_with_input(
            BenchmarkId::new("multipart_i64", format!("{}x{}", parts, rows_per_part)),
            &dir,
            |b, dir| {
                b.iter(|| {
                    let mut scan = FullScan::new(dir.path().to_path_buf(), col_def.clone()).unwrap();
                    while let Some(batch) = scan.next_batch() {
                        black_box(batch.unwrap());
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_single_col_pruned, bench_all_cols, bench_string_col, bench_multipart_scan);
criterion_main!(benches);
