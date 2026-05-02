# Current Tasks

---

## Deliberately Deferred

- Nullable columns (parallel presence bitmap, `.null.bin`/`.null.mrk`)
- INSERT / SELECT parsing and execution
- Background merging (Phase 2)
- Dictionary encoding — revisit alongside a `LowCardinality(String)` column type rather than as a generic codec for `String` columns.
- Encoding library (Plain, Delta, RLE) integration with column writers is still pending — pick up after the parser lands.

---

## Appendix: Future Optimizations

### A1 — Zero-copy buffering
Currently, each value is serialized byte-by-byte via `extend_le_bytes` into the block buffer. Once encodings are in place, explore eliminating this copy:
- With `bytemuck::cast_slice`, a `&[T]` can be reinterpreted as `&[u8]` directly (safe, no copy) and fed straight to the LZ4 compressor.
- The `IDataType` trait bound would be replaced or augmented with `bytemuck::Pod`.
- Only applies to the raw "no encoding" path; encoded paths produce their own byte streams.

### OQ1 — Adaptive granularity
Currently fixed 512-row granules for all column types. ClickHouse-style adaptive granularity would target a fixed byte size (e.g. 8 KiB uncompressed) per granule, varying row count by data width. Requires `TableWriter` computing boundaries upfront from the full batch (fixed bytes for numerics, `len+4` for strings) and column writers accepting a `write_granule(start, count)` API. Alignment across columns is preserved because boundaries are computed at the `TableWriter` level. Revisit once encodings land — encoded sizes vary per granule, which makes byte-targeted granularity more interesting.

### OQ2 — Auto-select encoding from chunk stats
Once 2+ encodings exist, a small heuristic can pick per chunk: scan the values, estimate encoded size for each candidate, pick the winner. Cheaper than it sounds at 512 values per granule.

### OQ3 — SIMD decode paths
Delta decode and dictionary lookup are both natural SIMD targets. Defer until SPEC §1.4 (SIMD in WHERE / aggregations) is on the table.
