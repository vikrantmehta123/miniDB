# Deliberately deferred
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
