# Current Task: Generic Numeric Column (IDataType + IColumn)

Extend the existing i64-only storage pipeline to support all numeric types. Introduce two traits — `IDataType` (implemented on Rust primitives) and `IColumn` (implemented on `ColumnVector<T>`) — and generalize the storage layer to work with any type that implements them. A CLI argument selects the type at runtime.

---

## Concepts

| Term | Definition |
|---|---|
| **IDataType** | A trait implemented directly on Rust primitive types (`i32`, `f64`, etc.). Knows the type name, byte size, and how to serialize/deserialize one value as little-endian bytes. |
| **IColumn** | A trait any column must implement. Knows its length and how to bulk-serialize/deserialize values. |
| **ColumnVector\<T\>** | A generic struct holding `Vec<T>` where `T: IDataType`. The only concrete column type for now. |

---

## Phase 1: IDataType trait

### Step 1 — Create `src/types.rs` ✓ DONE
- [x] Define the `IDataType` trait with four methods:
  - `fn name() -> &'static str` — e.g. `"Int64"`, `"Float32"`
  - `fn size_of() -> usize` — bytes per value
  - `fn to_le_bytes_vec(&self) -> Vec<u8>` — serialize one value to little-endian bytes
  - `fn from_le_bytes(bytes: &[u8]) -> Self` — deserialize one value from little-endian bytes
- [x] Implement `IDataType` for all ten numeric types: `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, `f64`

*Rust lesson: implementing a trait you own on types you don't own (primitives). Associated functions vs methods (`fn name()` has no `self`).*

---

## Phase 2: IColumn trait + ColumnVector

### Step 2 — Create `src/column.rs` ✓ DONE
- [x] Define the `IColumn` trait:
  - `fn len(&self) -> usize`
  - `fn serialize_binary_bulk(&self, buf: &mut Vec<u8>, offset: usize, limit: usize)`
  - `fn deserialize_binary_bulk(&mut self, buf: &[u8])`
- [x] Define `ColumnVector<T: IDataType>` with a single field: `data: Vec<T>`
- [x] Implement `IColumn` for `ColumnVector<T: IDataType>`:
  - `serialize_binary_bulk`: loop over `data[offset..offset+limit]`, call `.to_le_bytes_vec()`, extend buf
  - `deserialize_binary_bulk`: loop over `buf.chunks_exact(T::size_of())`, call `T::from_le_bytes()`, push to data

*Rust lesson: generic structs, trait bounds, `chunks_exact`.*

---

## Phase 3: Generalize storage

### Step 3 — Generalize `src/storage.rs` ✓ DONE
- [x] `write_column<T: IDataType>(col: &ColumnVector<T>)` — replaced hardcoded `* 8` with `T::size_of()`; calls `col.serialize_binary_bulk()` granule by granule
- [x] `read_granule<T: IDataType>(mark) -> Result<ColumnVector<T>>` — decompresses block, slices granule bytes, calls `deserialize_binary_bulk`
- [x] Marks format unchanged — still three little-endian `u64`s per mark (24 bytes), fully type-agnostic

*Rust lesson: generic functions with trait bounds, how type parameters flow through the call chain.*

---

## Phase 4: CLI wiring + round-trip verification

### Step 4 — Wire it up in `main.rs` ✓ DONE
- [x] Read `--type <name>` from `std::env::args()`
- [x] Match on the string, dispatch to a generic `run::<T>()` function:
  - `"i8"` → `run::<i8>()`, `"i16"` → `run::<i16>()`, ... all ten types
- [x] `run::<T>()`: generate 10 000 test values, write to disk, read every granule back, assert round-trip

*Rust lesson: `std::env::args()`, bridging runtime string → compile-time generic via match.*

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
