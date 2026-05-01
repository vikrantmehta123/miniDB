# Current Tasks

## Step 5 — TableWriter

Takes a `TableDef` and a table directory. On `create()`, allocates the next part directory (`part_00001`, `part_00002`, …) and opens one writer per column, dispatching on `DataType`:

```rust
match col.data_type {
    DataType::I64 => ColumnWriter::<i64>::create(...),
    DataType::Str => StringColumnWriter::create(...),
    // ...
}
```

API:

```rust
impl TableWriter {
    pub fn create(table_dir: &Path, def: &TableDef) -> std::io::Result<Self>;
    pub fn write_row(&mut self, row: &[Value]) -> std::io::Result<()>;
    pub fn flush(&mut self) -> std::io::Result<()>;
}
```

`Value` is a runtime enum parallel to `DataType` — it holds an actual value at runtime:

```rust
pub enum Value {
    I8(i8), I16(i16), I32(i32), I64(i64),
    U8(u8), U16(u16), U32(u32), U64(u64),
    F32(f32), F64(f64),
    Bool(bool),
    Str(String),
}
```

`write_row` takes a slice of `Value` (one per column), validates each value's variant matches the column's `DataType`, and pushes to the right writer.

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
