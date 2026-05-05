# TASK-007 — SIMD: Vectorized Scan and Aggregation

## Description
Use SIMD to vectorize the two hottest paths: predicate comparisons in `evaluator.rs` and sum accumulation in `aggregator/sum.rs`. This is listed as a **required** learning goal in SPEC.md — it's the "I understand the hardware" proof point in any systems interview.

---

## Steps

### Choose the SIMD approach (decide before writing code)

- [ ] **Option A**: `std::simd` (nightly Rust) — portable, idiomatic, unstable
- [ ] **Option B**: `wide` crate (stable) — stable API over the same SIMD ops
- [ ] **Option C**: raw `std::arch` intrinsics — maximum control, x86-specific
- [ ] Recommendation: start with `wide` (stable, readable); graduate to `std::arch` for a specific hot loop if you want to show depth
- [ ] Record decision here before starting

### Vectorized comparison (`src/evaluator.rs`)

- [ ] For `ColumnChunk::I64` with `Predicate::Cmp`: process 4 (or 8) values per iteration using a SIMD lane
- [ ] Compare lane vs a splat of the literal value, extract a `u64` bitmask
- [ ] Convert bitmask to `Vec<bool>` for the tail and for types not yet SIMD-accelerated
- [ ] Gate the SIMD path behind a type check or a separate function; keep the scalar fallback for all other types

### Vectorized sum (`src/aggregator/sum.rs`)

- [ ] For `Sum<i64>`: accumulate into a SIMD register (4 or 8 lanes), horizontal-add at the end of each chunk
- [ ] Scalar tail for `len % lane_width != 0`
- [ ] Add a separate `sum_simd` function; call it from `update` based on a feature flag or always

### Benchmark before/after

- [ ] Re-run the scan benchmark with a `WHERE ts > X` predicate — scalar vs SIMD
- [ ] Re-run `SELECT sum(ts) FROM events` — scalar vs SIMD
- [ ] Record numbers here

### Numbers (fill in after benchmarking)

| Path | Scalar MB/s | SIMD MB/s | Speedup |
|---|---|---|---|
| i64 `>` comparison | — | — | — |
| i64 sum | — | — | — |

---

## Out of Scope
- SIMD for string comparisons
- AVX-512 (focus on SSE4.2 / AVX2 first)
- SIMD for Delta decode (see OPTIMIZATIONS.md)
