use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use tempfile::tempdir;
use tinyolap::executor::execute_select;
use tinyolap::parser::{parse, Statement};
use tinyolap::parser::ast::SelectStmt;
use tinyolap::storage::column_chunk::ColumnChunk;
use tinyolap::storage::schema::{ColumnDef, DataType, TableDef};
use tinyolap::storage::table_writer::TableWriter;

const ROWS: usize = 1_000_000;

const EVENT_NAMES: &[&str] = &[
    "click", "view", "purchase", "scroll", "hover",
    "login", "logout", "search", "share", "bookmark",
];

fn schema() -> TableDef {
    TableDef {
        name: "events".to_string(),
        columns: vec![
            ColumnDef { name: "ts".to_string(),    data_type: DataType::I64 },
            ColumnDef { name: "uid".to_string(),   data_type: DataType::U32 },
            ColumnDef { name: "event".to_string(), data_type: DataType::Str },
            ColumnDef { name: "val".to_string(),   data_type: DataType::F64 },
        ],
        sort_key: vec![0],
    }
}

fn write_data(dir: &std::path::Path) {
    let schema = schema();
    TableDef::create(dir, &schema).unwrap();
    let writer = TableWriter::open(dir.to_path_buf()).unwrap();

    let ts: Vec<i64>        = (0..ROWS as i64).collect();
    let uid: Vec<u32>       = (0..ROWS as u32).map(|i| i % 100_000).collect();
    let events: Vec<String> = (0..ROWS)
        .map(|i| EVENT_NAMES[i % EVENT_NAMES.len()].to_string())
        .collect();
    let val: Vec<f64>       = (0..ROWS).map(|i| i as f64 * 0.001).collect();

    writer.insert(vec![
        ColumnChunk::I64(ts),
        ColumnChunk::U32(uid),
        ColumnChunk::Str(events),
        ColumnChunk::F64(val),
    ]).unwrap();
}

fn select_stmt(sql: &str) -> SelectStmt {
    match parse(sql).unwrap() {
        Statement::Select(s) => s,
        _ => panic!("expected SELECT"),
    }
}

fn bench_full_scan(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    write_data(dir.path());
    let schema = schema();
    let stmt = select_stmt("SELECT ts, uid, val FROM events");

    let mut group = c.benchmark_group("query");
    group.throughput(Throughput::Elements(ROWS as u64));
    group.bench_function("full_scan_no_filter", |b| {
        b.iter(|| {
            black_box(
                execute_select(stmt.clone(), &schema, dir.path().to_path_buf()).unwrap()
            );
        });
    });
    group.finish();
}

fn bench_filter_selectivity(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    write_data(dir.path());
    let schema = schema();

    // ts is 0..1M — threshold directly controls selectivity
    let configs = [
        ("1pct",  10_000i64),
        ("10pct", 100_000i64),
        ("50pct", 500_000i64),
    ];

    let mut group = c.benchmark_group("query");
    for (label, threshold) in configs {
        let stmt = select_stmt(&format!("SELECT ts, val FROM events WHERE ts < {}", threshold));
        group.throughput(Throughput::Elements(ROWS as u64));
        group.bench_function(BenchmarkId::new("filter_selectivity", label), |b| {
            b.iter(|| {
                black_box(
                    execute_select(stmt.clone(), &schema, dir.path().to_path_buf()).unwrap()
                );
            });
        });
    }
    group.finish();
}

fn bench_aggregate(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    write_data(dir.path());
    let schema = schema();
    let stmt = select_stmt("SELECT SUM(val), COUNT(val), AVG(val) FROM events");

    let mut group = c.benchmark_group("query");
    group.throughput(Throughput::Elements(ROWS as u64));
    group.bench_function("aggregate_no_group_by", |b| {
        b.iter(|| {
            black_box(
                execute_select(stmt.clone(), &schema, dir.path().to_path_buf()).unwrap()
            );
        });
    });
    group.finish();
}

fn bench_group_by(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    write_data(dir.path());
    let schema = schema();

    // low:  10 distinct groups  (event column — cycles over 10 values)
    // high: 100k distinct groups (uid column — uid % 100_000)
    let configs = [
        ("low_10_groups",       "SELECT event, SUM(val) FROM events GROUP BY event"),
        ("high_100k_groups",    "SELECT uid, SUM(val) FROM events GROUP BY uid"),
    ];

    let mut group = c.benchmark_group("query");
    group.throughput(Throughput::Elements(ROWS as u64));
    for (label, sql) in configs {
        let stmt = select_stmt(sql);
        group.bench_function(BenchmarkId::new("group_by", label), |b| {
            b.iter(|| {
                black_box(
                    execute_select(stmt.clone(), &schema, dir.path().to_path_buf()).unwrap()
                );
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_full_scan, bench_filter_selectivity, bench_aggregate, bench_group_by);
criterion_main!(benches);
