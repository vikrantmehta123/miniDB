# TASK-004 — Rayon: Parallel Part Scan

## Description
`rayon` is already in `Cargo.toml` but unused. Parallelize the part loop in `FullScan` so multiple parts are scanned concurrently. Each part is fully independent — no shared mutable state — making this a clean data-parallel problem with near-zero risk.

---

## Completed

- [x] **Parallelize part scan** (`src/processors/full_scan.rs`)
  - Eager parallel collect in `FullScan::new`: `par_iter()` over all part IDs, reads each part
    inside the closure (own file handles, no sharing), collects into `Vec<Batch>`, reversed so
    `pop()` yields parts in ascending order. `next_batch()` is a single `pop()`.

- [x] **Thread safety**
  - Each part opens its own `ColumnReader` / `StringColumnReader` inside the closure — no shared state.
  - `ColumnReader` holds only `File` + `Vec<u8>` cache, both `Send`. No `Rc` or `RefCell`.

- [x] **Preserve part order**
  - `par_iter()` on a slice is an `IndexedParallelIterator` — rayon's `collect()` preserves
    original index order. Reversed once after collection; `pop()` yields forward.

- [x] **Read entire `.bin` file in one shot** (`src/storage/column_reader.rs`, `string_column_reader.rs`)
  - Added `read_all()`: one `read_to_end()` syscall per column file, processes all blocks from
    the in-memory buffer. Reuses a single `decoded` buffer across blocks — no per-block allocation.
  - Renamed old granule-by-granule path to `read_granules()` to avoid confusion.
  - `FullScan` calls `read_all()` on every column. `read_granules()` kept for future selective
    scan operators (predicate pushdown, point lookups).

- [x] **Benchmark** — 100 parts × 10k rows (800 KiB total), 8-core machine

### Speedup Numbers

| Parts | Threads | Throughput | Speedup vs 1 thread |
|---|---|---|---|
| 100 | 1 | 267 MiB/s | 1.0× |
| 100 | 4 | 955 MiB/s | 3.6× |
| 100 | 8 | 1.07 GiB/s | 4.1× |

Scaling is sub-linear (4.1× on 8 cores) because parts are small (10k rows each) and I/O
overhead per part dominates at this size. Expected to scale closer to linear with larger parts.

---

## Out of Scope
- Column-level parallelism within a part (TASK-005)
- Async I/O
- Concurrent inserts
