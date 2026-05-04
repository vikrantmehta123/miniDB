# TASK-011 — SIMD: Vectorized Scan and Aggregation

## Description
Use SIMD intrinsics to vectorize the hot paths: predicate evaluation (comparisons) and aggregation (sum). This is listed as a **required** learning goal in SPEC.md and is the "I understand the hardware" proof point.

**Sprint 5 — estimated 2–3 sessions.**

---

## Steps

- [ ] **Choose the SIMD approach**
  - Option A: `std::simd` (nightly Rust) — portable, idiomatic, still unstable
  - Option B: `wide` crate (stable) — wraps `std::simd` with a stable API
  - Recommendation: use `wide` to avoid nightly dependency; revisit `std::simd` if stabilized
  - Decision to be recorded here before writing code

- [ ] **Vectorized comparison in the predicate evaluator** (`src/storage/column_reader.rs` or a new `src/simd/` module)
  - For `Predicate::Cmp` on an i64 column: load 4 (or 8) values at a time into a SIMD lane
  - Compute a SIMD comparison, extract a bitmask
  - Fall back to scalar for the tail (len % lane_width != 0)
  - Gate behind `#[cfg(target_arch = "x86_64")]` or use `wide`'s portable abstraction

- [ ] **Vectorized sum accumulation**
  - Replace the scalar `sum += v` loop in the `Sum` accumulator with a SIMD horizontal add
  - Final horizontal reduce at the end of the granule

- [ ] **Benchmark before/after**
  - Re-run TASK-002 scan benchmark and the WHERE-filtered scan from TASK-005
  - Report scalar vs SIMD throughput in MB/s
  - Record numbers here

---

## SIMD Throughput Numbers (fill in after benchmarking)

| Path | Scalar MB/s | SIMD MB/s | Speedup |
|---|---|---|---|
| i64 equality scan | — | — | — |
| i64 sum | — | — | — |

---

## Out of Scope
- SIMD string comparisons
- AVX-512 (focus on SSE4.2 / AVX2 first)
- SIMD for Delta decode (deferred — see OPTIMIZATIONS.md)
