# TASK-004 — Rayon: Parallel Part Scan

## Description
`rayon` is already in `Cargo.toml` but unused. Parallelize the part loop in `FullScan` so multiple parts are scanned concurrently. Each part is fully independent — no shared mutable state — making this a clean data-parallel problem with near-zero risk.

**Prerequisite: TASK-001 (zone maps change FullScan's granularity — do that first).**

---

## Steps

- [ ] **Parallelize part scan** (`src/processors/full_scan.rs`)
  - The current `FullScan` yields one part per `next_batch()` call — it's pull-based and inherently sequential
  - Switch to an eager parallel scan: at construction time, collect all part ids; in `next_batch()`, do a one-shot `par_iter()` over all parts, collecting results into a `Vec<Batch>` ordered by part index; then yield them sequentially from that vec
  - Alternatively, keep the pull model and use a background thread pool — simpler approach first
  - Simplest correct approach: eager parallel collect in `FullScan::new` or on first `next_batch()` call; store results in `self.batches: Vec<Batch>`; subsequent calls drain the vec

- [ ] **Thread safety**
  - Each part opens its own file handles inside the closure — no sharing needed
  - `ColumnReader` must be `Send` — verify no `Rc` or `RefCell` crosses the boundary

- [ ] **Preserve part order**
  - Use `rayon::iter::IndexedParallelIterator::collect()` or sort by part_id after collection
  - Row order must match the sequential baseline

- [ ] **Benchmark**
  - Re-run the TASK-002 scan benchmark on a dataset with 10+ parts
  - Compare `RAYON_NUM_THREADS=1` vs default; record speedup here

### Speedup Numbers (fill in after benchmarking)

| Parts | Threads | Throughput | Speedup vs 1 thread |
|---|---|---|---|
| 10 | 1 | — | — |
| 10 | 4 | — | — |
| 10 | 8 | — | — |

---

## Out of Scope
- Column-level parallelism within a part (TASK-005)
- Async I/O
- Concurrent inserts
