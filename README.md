# tinyOLAP

A columnar database built from scratch in Rust, inspired by ClickHouse. This is a learning project — not a production system. 

## Features

### Storage
- **Columnar layout** — data stored column-by-column; reads touch only the columns a query needs
- **LZ4 compression** — every column file is compressed; decompressed on read
- **Granule-based indexing** — data is divided into granules of 512 values; each granule has a mark (byte offset) so reads can seek directly to the right block without a full scan
- **Immutable parts** — each `INSERT` produces an atomic, crash-safe part directory written to `tmp_part_NNNNN/` and renamed on success
- **Sorted parts** — rows are sorted by the primary key column(s) before being written
- **Encoding codecs** — Plain, Delta, and RLE encoders available per column

### Supported Types
`i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, `f64`, `bool`, variable-length strings

### Query Processing
- **Full table scan** — `SELECT col1, col2 FROM table` and `SELECT * FROM table`; only requested columns are read
- **WHERE clause** — predicates with `=`, `!=`, `<`, `<=`, `>`, `>=`, `AND`, `OR`, `NOT`
- **Aggregations** — `SUM`, `AVG`, `COUNT`, `MIN`, `MAX`
- **GROUP BY** — group aggregations by one or more columns
- **Parallel scans** — Parts are read in parallel via `rayon`

### SQL Dialect
```sql
INSERT INTO <table> VALUES (...), (...);
SELECT col, agg(col) FROM <table> WHERE <predicate> GROUP BY col;
```

---

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)

### Clone and build

```bash
git clone https://github.com/vikrantmehta123/tinyOLAP.git
cd tinyOLAP
cargo build
```

### Create a schema

tinyOLAP reads a `schema.json` from the table directory at startup. Create one before running:

```bash
mkdir -p data/my_table
cat > data/my_table/schema.json << 'EOF'
{
  "name": "my_table",
  "columns": [
    { "name": "ts",    "data_type": "I64" },
    { "name": "uid",   "data_type": "U32" },
    { "name": "value", "data_type": "F64" },
    { "name": "tag",   "data_type": "Str" }
  ],
  "sort_key": [0]
}
EOF
```

`sort_key` is a list of column indices (zero-based) that form the primary key. The default table directory is `data/tinyolap_smoke`. To use a different directory, edit `src/main.rs`.

### Run

```bash
cargo run
```

```
tinyOLAP ready. Table: 'my_table'
Type SQL and press Enter. Ctrl-D to quit.

> INSERT INTO my_table VALUES (1700000000, 1, 9.5, 'cpu'), (1700000060, 2, 3.1, 'mem');
OK (2 rows inserted, part_0)

> SELECT ts, tag FROM my_table WHERE uid = 1;
...

> SELECT tag, SUM(value) FROM my_table GROUP BY tag;
...
```

### Run tests

```bash
cargo test
```

---

## Project Layout

```
src/
  storage/      # column writers, readers, marks, parts, schema
  encoding/     # Plain, Delta, RLE codecs
  parser/       # SQL → internal AST (via sqlparser-rs)
  processors/   # query pipeline stages: scan, filter, project, aggregate, group-by
  aggregator/   # SUM, COUNT, AVG, MIN, MAX implementations
  analyser.rs   # semantic analysis / query planning
  executor.rs   # ties the pipeline together
  main.rs       # REPL entry point
tasks/          # per-task markdown files tracking active work
SPEC.md         # phase 1 / phase 2 feature roadmap
```

---

## Inspiration

Architecture decisions — granule size, mark files, immutable parts, LZ4 — are drawn from studying the [ClickHouse](https://github.com/ClickHouse/ClickHouse) source.
