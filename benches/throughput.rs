use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::hint::black_box;
use tempfile::tempdir;
use tinyolap::processors::full_scan::FullScan;
use tinyolap::processors::processor::Processor;
use tinyolap::storage::column_chunk::ColumnChunk;
use tinyolap::storage::schema::{ColumnDef, DataType, TableDef};
use tinyolap::storage::table_writer::TableWriter;

const N: usize = 1_000_000;

fn schema() -> TableDef {
    TableDef {
        name: "bench".to_string(),
        columns: vec![ColumnDef { name: "ts".to_string(), data_type: DataType::I64 }],
        sort_key: vec![0],
    }
}

fn bench_write(c: &mut Criterion) {
    let data: Vec<i64> = (0..N as i64).collect();
    let bytes = (N * std::mem::size_of::<i64>()) as u64;

    let mut group = c.benchmark_group("write");
    group.throughput(Throughput::Bytes(bytes));
    group.bench_function("1M i64", |b| {
        b.iter(|| {
            let dir = tempdir().unwrap();
            TableDef::create(dir.path(), &schema()).unwrap();
            let writer = TableWriter::open(dir.path().to_path_buf()).unwrap();
            writer.insert(vec![ColumnChunk::I64(black_box(data.clone()))]).unwrap();
        });
    });
    group.finish();
}

fn bench_scan(c: &mut Criterion) {
    let data: Vec<i64> = (0..N as i64).collect();
    let bytes = (N * std::mem::size_of::<i64>()) as u64;

    let dir = tempdir().unwrap();
    TableDef::create(dir.path(), &schema()).unwrap();
    let writer = TableWriter::open(dir.path().to_path_buf()).unwrap();
    writer.insert(vec![ColumnChunk::I64(data)]).unwrap();

    let col_def = vec![ColumnDef { name: "ts".to_string(), data_type: DataType::I64 }];

    let mut group = c.benchmark_group("scan");
    group.throughput(Throughput::Bytes(bytes));
    group.bench_function("1M i64", |b| {
        b.iter(|| {
            let mut scan = FullScan::new(dir.path().to_path_buf(), col_def.clone()).unwrap();
            while let Some(batch) = scan.next_batch() {
                black_box(batch.unwrap());
            }
        });
    });
    group.finish();
}

criterion_group!(benches, bench_write, bench_scan);
criterion_main!(benches);
