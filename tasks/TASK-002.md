# TASK-002 — Criterion Benchmarks

## Description
Wire up `criterion` and establish baseline throughput numbers for the write and scan paths. These numbers are the "proof" behind every performance claim on a resume. Record them in this file so future comparisons are honest.

**Sprint 1 — estimated 1 session.**

---

## Steps

- [ ] **Add criterion**
  - Add `criterion` to `[dev-dependencies]` in `Cargo.toml`
  - Add a `[[bench]]` entry pointing to `benches/throughput.rs`

- [ ] **Write-path benchmark**
  - Generate 1M i64 values (e.g. `0..1_000_000`)
  - Benchmark `TableWriter::insert` end-to-end (includes LZ4 + fsync)
  - Report throughput in MB/s and ns/value

- [ ] **Scan-path benchmark**
  - Pre-write the same 1M rows to a temp directory in a `setup` closure
  - Benchmark `TableReader` full scan of one i64 column
  - Report throughput in MB/s

- [ ] **Record baseline numbers here**

---

## Baseline Numbers (fill in after running)

| Benchmark | Throughput | Notes |
|---|---|---|
| write 1M i64 | ~705 MiB/s | Delta + LZ4 + fsync, ~10.8 ms/iter |
| scan 1M i64 | ~443 MiB/s | LZ4 decompress + delta decode, ~17.2 ms/iter |

---

## Out of Scope
- Per-codec benchmarks (add in TASK-006 monomorphization evaluation if revisited)
- Multi-column benchmarks
