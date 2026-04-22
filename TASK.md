# Current Task: ClickHouse-style Compressed Storage + Marks (i64 column)

Implement compressed, block-based storage for a single `i64` column, following ClickHouse's MergeTree storage model. Data is buffered, compressed with LZ4, and written in blocks. A parallel marks file tracks how to seek into compressed blocks at granule granularity.

---

## Concepts (read first)

| Term | Definition |
|---|---|
| **Granule** | 512 consecutive `i64` values — the smallest unit of data you can skip over |
| **Compressed block** | One LZ4-compressed payload, produced from a full 8 KB uncompressed buffer |
| **Mark** | A triplet `(block_offset, granule_offset_in_decompressed, num_rows)` — enough to seek to any granule without decompressing earlier blocks |

Each compressed block contains one or more complete granules (8 KB / 512 values × 8 bytes/value = exactly 2 granules per block, if the buffer fills completely). The final block may be partial.

---

## Phase 1: Buffer and compress

### Step 1 — Simulate input + set up constants ✓
- [x] In `src/storage.rs`, define `GRANULE_SIZE: usize = 512` and `BLOCK_BUFFER_SIZE: usize = 8 * 1024`.
- [x] Generate a simulated `Vec<i64>` of 10 000 values in `main.rs`.

### Step 2 — Buffer values and flush compressed blocks ✓
- [x] Accumulate `i64` values as little-endian bytes into a `Vec<u8>` buffer.
- [x] When buffer reaches `BLOCK_BUFFER_SIZE` or input is exhausted, LZ4-compress and write to `column.bin`.
- [x] Each block is preceded by a `u32` compressed-length header so it can be read back without knowing the size in advance.
- [x] Track running `block_offset` (including the 4-byte header).

### Step 3 — Generate marks while writing ✓
- [x] `Mark { block_offset, granule_offset, num_rows }` emitted at the start of every granule (`rows_in_buffer % GRANULE_SIZE == 0`).
- [x] `granule_offset` is the byte position of the granule within the uncompressed block.
- [x] Final mark's `num_rows` fixed up via `total % GRANULE_SIZE`.

### Step 4 — Write marks file ✓
- [x] Each mark serialized as three little-endian `u64`s (24 bytes) into `column.mrk`.

---

## Phase 2: Verify round-trip

### Step 5 — Read back and decompress ✓
- [x] `read_mark(index)` — seeks to `index * 24` in `column.mrk`, reads 24 bytes, deserializes.
- [x] `read_granule(mark)` — seeks to `block_offset`, reads compressed-length header, reads + decompresses block, slices out granule bytes, casts to `Vec<i64>`.

*Rust lesson: `Seek`, `Read`, `lz4_flex::decompress_size_prepended`.*

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
Currently, each `i64` value is copied from the input slice into the `Vec<u8>` buffer via `extend_from_slice`. Once the core functionality is stable and correct, explore eliminating this copy:
- The input `&[i64]` is already a contiguous block of memory. With `bytemuck::cast_slice`, it can be reinterpreted as `&[u8]` directly (safe, no copy) and fed straight to the LZ4 compressor.
- This only works cleanly when the input fits exactly into block boundaries; partial blocks at the end still need care.
- **Do this only after the read/write round-trip is verified correct.**
