# TASK-006 — Rayon: Part-Level Parallelism

## Description
Parallelize the outer part loop in `TableReader` using `rayon`. Each part is independent — no shared mutable state — making this a clean data-parallel problem. After this, scan throughput scales linearly with available cores.

**Sprint 3 — estimated 1 session.**

---

## Steps

- [ ] **Add rayon** to `[dependencies]` in `Cargo.toml`

- [ ] **Parallelize part scan** (`src/storage/table_reader.rs`)
  - Replace the sequential `for part in parts` loop with `parts.par_iter()`
  - Each part produces a `Vec<ColumnChunk>`; collect into a `Vec` ordered by part index
  - Use `rayon::iter::IndexedParallelIterator` to preserve part order in the result

- [ ] **Thread safety audit**
  - Confirm no `Rc`, `Cell`, or non-`Send` types cross the parallel boundary
  - `File` handles are opened inside the closure — each thread opens its own, no sharing needed

- [ ] **Benchmark**: re-run the TASK-002 scan benchmark on a multi-part dataset, compare 1 vs N cores (set `RAYON_NUM_THREADS=1` vs default)

---

## Out of Scope
- Granule-level parallelism within a part (TASK-007)
- Concurrent inserts (not in Phase 1)
