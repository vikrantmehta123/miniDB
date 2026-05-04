use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::hint::black_box;
use tempfile::tempdir;
use tinyolap::storage::column_chunk::ColumnChunk;
use tinyolap::storage::schema::{ColumnDef, DataType, TableDef};
use tinyolap::storage::table_reader::TableReader;
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
        let reader = TableReader::open(dir.path()).unwrap();
        b.iter(|| {
            black_box(reader.read_all(&col_def).unwrap());
        });
    });
    group.finish();
}

criterion_group!(benches, bench_write, bench_scan);
criterion_main!(benches);
