# Current Task: Columnar Abstraction

Build a `Column` abstraction that works with all numeric types (`i8`–`i64`, `u8`–`u64`, `f32`, `f64`), `bool`, and fixed-size strings. Disk-backed, chunked (1024 values/chunk).

---

## Phase 1: Single-type `Column` struct (concrete `i64`)

### Step 1 — Extract a `Column` struct ✓
- [x] Create `src/column.rs`, declare `mod column;` in `main.rs`.
- [x] Define `struct Column { path: PathBuf, num_rows: usize }` (hardcoded to `i64` for now).
- [x] Move `write_column` / `read_chunks` into `impl Column` as `append_chunk(&mut self, &[i64])` and `read_chunk(&self, idx) -> Option<Vec<i64>>`.
- [x] Fix the EOF bug: `read_chunk` uses `num_rows` to bound reads and returns `None` past the end.
- [x] `main.rs` becomes a small driver that constructs a `Column` and calls methods.

*Rust lesson: modules, `pub`, `&mut self` vs `&self`, `Option` vs `Result`.*

---

## Phase 2: The type system

### Step 2 — Define `DataType` enum
- [ ] New `src/types.rs`. Variants: `I8, I16, I32, I64, U8, U16, U32, U64, F32, F64, Bool, FixedString(u16)`.
- [ ] Add method `fn size_bytes(&self) -> usize` — per-value byte width (8 for `i64`, 1 for `bool`, `n` for `FixedString(n)`).
- [ ] No `Value` enum yet — don't need it until we query.

*Rust lesson: enums with data (`FixedString(u16)`), methods on enums.*

### Step 3 — Make `Column` type-aware
- [ ] Add `dtype: DataType` field to `Column`.
- [ ] Rewrite `append_chunk` and `read_chunk` to take/return `&[u8]` and `Vec<u8>` (raw bytes).
- [ ] Offset math uses `self.dtype.size_bytes()` instead of hardcoded `8`.

*Key insight: columnar storage on disk is just typed bytes. The type only matters when you interpret them.*

---

## Phase 3: Typed read/write helpers

### Step 4 — Typed append/read per numeric type
Pick one approach:
- **(a)** Separate methods per type: `append_i64`, `read_chunk_i64`, etc. Verbose but clear.
- **(b)** `trait ColumnType: Copy { const DTYPE: DataType; }`, generic `append<T: ColumnType>(&[T])` using `bytemuck::cast_slice`.

Recommended path: do **(a)** for `i64` + `f64` first, feel the repetition, then refactor to **(b)**.

- [ ] Implement typed helpers for `i64` and `f64`.
- [ ] Refactor into a `ColumnType` trait covering all numeric types.

*Rust lesson: traits with associated constants, `bytemuck` for safe transmute, generic bounds.*

### Step 5 — `bool` support
- [ ] Store as 1 byte per bool (simple first; bit-packing later).
- [ ] `append_bool` / `read_chunk_bool`.

### Step 6 — `FixedString(n)` support
- [ ] `append_str(&mut self, &[&str])` — pad/truncate each string to exactly `n` bytes (null-padded).
- [ ] `read_chunk_str(idx) -> Vec<String>` — slice bytes into `n`-sized chunks, trim trailing zeros, `String::from_utf8`.

*Rust lesson: `&str` vs `String`, UTF-8 validation, slice operations.*

---

## Phase 4: Prove it works

### Step 7 — Tests
- [ ] `#[cfg(test)] mod tests` in `column.rs`.
- [ ] Round-trip test per type: write N values, read chunk-by-chunk, assert equality.
- [ ] Partial-final-chunk test (N not a multiple of 1024).

### Step 8 — Demo in `main.rs`
- [ ] Create one column of each type, write, read back, print.

---

## Deliberately deferred
- **Metadata persistence** — `num_rows` and `dtype` not saved to disk yet (Phase 5: catalog).
- **Multiple columns as a table** — also catalog.
- **Compression, bit-packing, nulls** — optimizations, later.
- **Variable-length strings** — fixed-size only for now; `VARCHAR` much later.
