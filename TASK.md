# Current Tasks

## Active — SQL parser frontend (INSERT only)

Wire up a minimal SQL parser so we can drive `TableWriter::insert` from a SQL string end-to-end. Uses `sqlparser-rs` for the actual parsing; we own a thin internal AST so the executor never touches sqlparser types.

**Steps**
1. Add `sqlparser` dep to `Cargo.toml`.
2. `src/parser/ast.rs` — define `Statement`, `InsertStmt`, `Literal` enums.
3. `src/parser/lower.rs` — match on `sqlparser::ast::Statement::Insert(...)`, walk `SetExpr::Values { rows, .. }`, lower each `Expr` to our `Literal`. Reject everything else with `ParseError::Unsupported`.
4. `src/parser/mod.rs` — public `parse(sql: &str) -> Result<Statement, ParseError>`; calls sqlparser then `lower::lower`.
5. `mod parser;` in `main.rs`; smoke-test with a batch INSERT string.
6. Type-check + transpose `Vec<Vec<Literal>>` → `Vec<ColumnChunk>` against the schema. (Lives at the boundary between parser output and `TableWriter::insert`. Could go in `parser/` or a new `query/` module — decide when we get there.)

**Out of scope for this task:** SELECT, WHERE, expressions in VALUES beyond literals + unary minus, multi-statement scripts.

Encoding library (Plain, Delta, RLE) integration with column writers is still pending — pick up after the parser lands.

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
- Dictionary encoding — revisit alongside a `LowCardinality(String)` column type rather than as a generic codec for `String` columns.

---

## Appendix: Future Optimizations

### A1 — Zero-copy buffering
Currently, each value is serialized byte-by-byte via `extend_le_bytes` into the block buffer. Once encodings are in place, explore eliminating this copy:
- With `bytemuck::cast_slice`, a `&[T]` can be reinterpreted as `&[u8]` directly (safe, no copy) and fed straight to the LZ4 compressor.
- The `IDataType` trait bound would be replaced or augmented with `bytemuck::Pod`.
- Only applies to the raw "no encoding" path; encoded paths produce their own byte streams.
