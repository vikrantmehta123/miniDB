# Current Task: Storage Layer Restructuring

Restructure `src/storage.rs` for clarity and extensibility. Extract `Mark` into its own file, and introduce `ColumnWriter` and `ColumnReader` structs to replace the current free functions.

---

## Previously completed (Generic Numeric Column)
- `src/data_type.rs` (formerly `types.rs`): `IDataType` trait + impls for all 10 numeric primitives ✓
- `src/column.rs`: `IColumn` trait + `ColumnVector<T: IDataType>` ✓
- `src/storage.rs`: `write_column<T>` and `read_granule<T>` with LZ4 + marks ✓
- `src/main.rs`: `--type` CLI dispatch + round-trip verification for all 10 types ✓

---

## Phase 5: Restructure storage

### Step 1 — Create `src/mark.rs` ✓ DONE
- [x] Move `Mark` struct (3 `u64` fields) out of `storage.rs` into `src/mark.rs`
- [x] Add `to_bytes() -> [u8; 24]` and `from_bytes(bytes: &[u8]) -> Mark` on `Mark`
- [x] `MarkWriter { file: File, buf: Vec<u8> }` with `write(&Mark)`, `flush() -> Result<()>`
- [x] `MarkReader { file: File }` with `read_all() -> Result<Vec<Mark>>`
- [x] `storage.rs` uses `MarkWriter` inside `write_column`; `main.rs` uses `MarkReader` to load marks after writing

### Step 2 — Introduce `ColumnWriter<T>` and `ColumnReader` in `src/storage.rs` ✓ DONE
- [x] `ColumnWriter<T: IDataType> { col: ColumnVector<T>, mark_writer: MarkWriter, data_file: File }`
  - `push(&mut self, val: T)`: appends a value into the internal column
  - `flush(&mut self) -> Result<()>`: granule loop → `serialize_binary_bulk` → LZ4 compress → write block → `mark_writer.write(mark)`
- [x] `ColumnReader { mark_reader: MarkReader, data_file: File, block_cache: Option<(u64, Vec<u8>)> }` (no struct-level generic — type only at method level)
  - `read_granule<T: IDataType>(&mut self, mark: &Mark) -> Result<ColumnVector<T>>`: check cache by block_offset → on miss: seek → read compressed bytes → decompress → cache; slice granule from decompressed bytes → deserialize
  - `read_all<T: IDataType>(&mut self) -> Result<Vec<ColumnVector<T>>>`: `mark_reader.read_all()` then `read_granule` for each
- [x] Update `main.rs` write path to use `ColumnWriter`
- [x] Update `main.rs` read path to use `ColumnReader`

---

## Deliberately deferred
- Multi-type columns (back-burner until this storage layer is solid)
- Index / primary key lookups
- Merging multiple parts (MergeTree merge step)
- Compression codecs beyond LZ4 (ZSTD, Delta, DoubleDelta)
- Null handling / sparse columns

---

## Appendix: Future Optimizations

### A1 — Zero-copy buffering
Currently, each value is serialized byte-by-byte via `to_le_bytes_vec()` into the buffer. Once correct for all types, explore eliminating this copy:
- With `bytemuck::cast_slice`, a `&[T]` can be reinterpreted as `&[u8]` directly (safe, no copy) and fed straight to the LZ4 compressor.
- The `IDataType` trait bound would be replaced or augmented with `bytemuck::Pod`.
- This only works cleanly when the input fits exactly into block boundaries; partial blocks still need care.
- **Do this only after the read/write round-trip is verified correct for all numeric types.**
