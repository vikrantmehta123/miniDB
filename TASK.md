# Current Tasks

## Step 5 — TableWriter INSERT API

`TableWriter` has `create()`, `WriterBox`, and `flush()` implemented. The remaining piece is the insert API.

### What's done
- `TableWriter::create(table_dir)` — reads schema, allocates next part dir, opens one `WriterBox` per column
- `WriterBox::flush()` — dispatches to the right column writer
- `TableWriter::flush()` — iterates writers and flushes each

### Design decision
The storage layer receives **column-oriented batches** — the caller (executor/parser) is responsible for transposing rows into columns before calling insert. This keeps the storage layer simple and cache-friendly.

`INSERT INTO` = one part. The unit of work is a full batch, not a single row.

### What's needed

1. Add `Value` enum to `schema.rs` (mirrors `DataType` — one variant per type, holds the actual value).

2. Add `WriterBox::push_column(vals: Vec<Value>)` — iterates the vec and pushes each value into the inner writer. Returns a type-mismatch error if a `Value` variant doesn't match the box variant.

3. Add `TableWriter::insert(columns: Vec<Vec<Value>>)` — validates column count, then for each column calls `writers[j].push_column(columns[j])`, then calls `flush()`.

4. Write a round-trip test in `main.rs`: define a schema, create a `TableWriter`, call `insert`, then read back with `TableReader` (Step 6) and assert values match.

---

## Step 6 — TableReader

Opens an existing part directory and reads column data back out.

```rust
impl TableReader {
    pub fn open(part_dir: &Path, def: &TableDef) -> std::io::Result<Self>;
    pub fn read_column(&mut self, col_index: usize) -> std::io::Result<Vec<Value>>;
}
```

`read_column` dispatches on `DataType`, calls the right `ColumnReader::read_all<T>` or `StringColumnReader::read_all`, and wraps each value in a `Value` variant.

End goal: a round-trip test in `main.rs` that writes a multi-column table via `TableWriter` and reads it back via `TableReader`.

---

## Open Questions / Future Considerations

### OQ1 — Parallel column writes
Each column writer is independent — no shared state between them. This means `flush()` could dispatch each `WriterBox` to a separate thread (e.g. via `rayon`). Worth doing once single-threaded correctness is verified.

### OQ2 — Adaptive granularity
Currently using fixed 512-row granules for all column types. ClickHouse-style adaptive granularity would target a fixed byte size (e.g. 8KB uncompressed) per granule, varying the row count based on data width. This requires: (1) `TableWriter` computing granule boundaries upfront from the full batch by accumulating bytes per row (fixed for numerics, `len+4` for strings), (2) column writers accepting a `write_granule(start, count)` API instead of `push`. Alignment across columns is preserved because boundaries are computed at the `TableWriter` level and applied uniformly. Implement after the basic round-trip (Steps 5–6) is verified correct.

---

## Deliberately Deferred

- Nullable columns (parallel presence bitmap, `.null.bin`/`.null.mrk`)
- Parts / sorted insert batches
- INSERT / SELECT parsing and execution
- Background merging (Phase 2)

---

## Appendix: Future Optimizations

### A1 — Zero-copy buffering
Currently, each value is serialized byte-by-byte via `to_le_bytes_vec()` into the buffer. Once correct for all types, explore eliminating this copy:
- With `bytemuck::cast_slice`, a `&[T]` can be reinterpreted as `&[u8]` directly (safe, no copy) and fed straight to the LZ4 compressor.
- The `IDataType` trait bound would be replaced or augmented with `bytemuck::Pod`.
- This only works cleanly when the input fits exactly into block boundaries; partial blocks still need care.
- **Do this only after the read/write round-trip is verified correct for all numeric types.**
