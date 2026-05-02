# Current Tasks

## Active: Step 7 — Encoding schemes

Today every column is written as raw little-endian bytes → lz4 block. We want to add **logical encodings** that exploit data shape *before* lz4. Approach: build the encoding library standalone first with a stable API, then integrate with column writers in a separate step.

### Settled design decisions

1. **Per-block encoding, ClickHouse Wide-format style.** Codec is invoked once per compressed block, not per granule. Multiple granules accumulate into the block buffer; when buffer ≥ threshold *and* on a granule boundary, the whole buffer is encoded → lz4 → written. Confirmed against ClickHouse `MergeTreeDataPartWriterWide.cpp:445-447` and the `min_compress_block_size = 65536` default.
2. **Pipeline:** typed values → block buffer (raw) → encode (whole buffer) → lz4 → disk. Reader reverses. lz4 stays.
3. **Block buffer raw threshold raised to 32 KiB** (from 8 KiB). Encoding shrinks data before lz4 sees it; raising the raw threshold keeps the lz4 input size in its sweet spot. Pick a const, retune later if benchmarks demand.
4. **Granules stay at 512 values.** They are logical units — one mark per granule. Multiple granules per compressed block is fine and expected.
5. **Marks keep their current shape:** `(offset_in_compressed_file, offset_in_decompressed_block)`. The byte-offset-into-decompressed-block field is needed because granules share blocks. No mark format change needed for encoding.
6. **Encoding tag stored at the start of each compressed block** in `.bin` (one byte). Keeps marks unchanged and lets a single column mix encodings across blocks.
7. **Subdirectory layout:** `src/encoding/` with `mod.rs` (sealed `Primitive` trait, `Codec` enum, `EncodingError`, dispatch), one file per scheme. Enum-based dispatch over trait objects — closed set, exhaustive matching, no allocation.
8. **Selection policy:** explicit per-column choice in schema. Auto-selection deferred to OQ2.

### Library API (stable target)

```rust
pub trait Primitive: Copy + sealed::Sealed { /* le_bytes, wrapping ops */ }
pub enum Codec { Plain, Delta }
pub enum EncodingError { Truncated, BadHeader }
impl Codec {
    fn encode<T: Primitive>(self, src: &[T], out: &mut Vec<u8>);
    fn decode<T: Primitive>(self, src: &[u8], out: &mut Vec<T>) -> Result<(), EncodingError>;
}
```

`Primitive` is sealed and will cover integer types only at first (`i8/i16/i32/i64/u8/u16/u32/u64`). Floats join when a float-friendly codec lands.

### Build approach

Bottom-up: write the simplest concrete thing first, let real pain pull in abstractions. No `Primitive` trait, no `Codec` enum, no `EncodingError` until something actually needs them. The "Library API" above is the *eventual* shape, not a starting point.

### Done so far

- `src/encoding/mod.rs` — declares `pub mod delta;`. Nothing else yet.
- `src/encoding/delta.rs` — `encode`/`decode` for `&[i32]` ↔ `Vec<u8>`, using `wrapping_sub`/`wrapping_add` for overflow safety. Format: `[first : i32 LE][delta_1 : i32 LE]...`.
- Roundtrip tests in `delta.rs`: basic, empty, single value, extreme overflow (`i32::MAX ↔ i32::MIN`). All passing.

### Next steps (in order)

1. **Extend `delta` to more integer widths** (`i64`, `u32`, `u64`, etc.). The first attempt should deliberately copy-paste — write `encode_i64`/`decode_i64` next to the i32 versions. The duplication is the point: it's what motivates the next step.
2. **Introduce a sealed `Primitive` trait** in `mod.rs` once duplication hurts. Trait covers `to_le_bytes`/`from_le_bytes` and `wrapping_sub`/`wrapping_add`. Macro-generated impls for the integer widths. Collapse the duplicated `delta::encode_*` into one generic `encode<T: Primitive>`.
3. **Add `plain` encoding** (`plain.rs`) — identity baseline. Generic from the start since `Primitive` already exists by then. Useful as a default and as a sanity check on the trait machinery.
4. **Introduce `Codec` enum + `EncodingError`** in `mod.rs` once there are two codecs to dispatch between. Methods `Codec::encode`/`Codec::decode` over `T: Primitive`. Decode returns `Result<(), EncodingError>` — start with just `Truncated` and `BadHeader`; add variants on demand.
5. **Format-lock test** in `delta.rs` (and `plain.rs`) — assert exact byte layout for a fixed input, so an accidental endianness/header change is caught loudly.
6. *(separate task, after library is stable)* **Integration**: thread `Codec` choice into column writers, encode-then-lz4 at `flush_block`, write a 1-byte encoding tag at block start, reverse on read. Bump `BLOCK_BUFFER_SIZE` raw threshold to 32 KiB.
7. **Dictionary** for `Str` columns — deferred until integers are working end-to-end and integration is in place.
8. **RLE / DoubleDelta** — deferred.

### Why this order

Each abstraction is justified by a duplication or dispatch problem that already exists when it's introduced — no speculative scaffolding. The `Primitive` trait appears when we already have N copies of nearly-identical code; the `Codec` enum appears when we already have two codecs to switch between.

### Open question

- The encoding tag byte plus a possibly-changed block layout breaks compatibility with existing parts under `data/`. Since this is a learning project, plan is to delete `data/` rather than version the on-disk format. Confirm before shipping the integration step.

---

## Open Questions / Future Considerations

### OQ1 — Adaptive granularity
Currently fixed 512-row granules for all column types. ClickHouse-style adaptive granularity would target a fixed byte size (e.g. 8 KiB uncompressed) per granule, varying row count by data width. Requires `TableWriter` computing boundaries upfront from the full batch (fixed bytes for numerics, `len+4` for strings) and column writers accepting a `write_granule(start, count)` API. Alignment across columns is preserved because boundaries are computed at the `TableWriter` level. Revisit once encodings land — encoded sizes vary per granule, which makes byte-targeted granularity more interesting.

### OQ2 — Auto-select encoding from chunk stats
Once 2+ encodings exist, a small heuristic can pick per chunk: scan the values, estimate encoded size for each candidate, pick the winner. Cheaper than it sounds at 512 values per granule.

### OQ3 — SIMD decode paths
Delta decode and dictionary lookup are both natural SIMD targets. Defer until SPEC §1.4 (SIMD in WHERE / aggregations) is on the table.

---

## Deliberately Deferred

- Nullable columns (parallel presence bitmap, `.null.bin`/`.null.mrk`)
- INSERT / SELECT parsing and execution
- Background merging (Phase 2)

---

## Appendix: Future Optimizations

### A1 — Zero-copy buffering
Currently, each value is serialized byte-by-byte via `extend_le_bytes` into the block buffer. Once encodings are in place, explore eliminating this copy:
- With `bytemuck::cast_slice`, a `&[T]` can be reinterpreted as `&[u8]` directly (safe, no copy) and fed straight to the LZ4 compressor.
- The `IDataType` trait bound would be replaced or augmented with `bytemuck::Pod`.
- Only applies to the raw "no encoding" path; encoded paths produce their own byte streams.
