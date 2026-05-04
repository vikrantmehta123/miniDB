# tinyOLAP

A columnar database built from scratch in Rust, inspired by ClickHouse.

This is a learning project to understand how real column stores work - and not a production-grade database.

## Features

- **Columnar storage** — data stored column-by-column, compressed with LZ4
- **Granule-based indexing** — data is addressable in granules of 512 values, each backed by a mark file for fast seeking
- **Supported Data Types** — `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, `f64`, `bool`, and variable-length strings
- **Encoding schemes** — Plain, Delta, and RLE codecs per column
- **Immutable parts** — each INSERT produces an atomic, immutable part directory; writes are crash-safe.
- **Sorted Parts** — rows are sorted by the primary key column(s) before being written in parts.
- **SQL parsing** — `INSERT INTO` and `SELECT` via [sqlparser-rs](https://github.com/apache/arrow-datafusion/tree/main/datafusion/sql)
- **Full table scan** — `SELECT col1, col2 FROM table` and `SELECT * FROM table`; only requested columns are read from disk

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
    { "name": "ts",  "data_type": "I64" },
    { "name": "uid", "data_type": "U32" },
    { "name": "ok",  "data_type": "Bool" },
    { "name": "tag", "data_type": "Str" }
  ],
  "sort_key": [0]
}
EOF
```

The default table directory is `data/tinyolap_smoke`. To use a different directory, edit `main.rs`.

### Run

```bash
cargo run
```

```
tinyOLAP ready. Table: 'my_table'
Type SQL and press Enter. Ctrl-D to quit.

> INSERT INTO my_table VALUES (1700000000, 1, true, 'hello'), (1700000060, 2, false, 'world');
OK (2 rows inserted, part_0)

> SELECT ts, tag FROM my_table;
```
