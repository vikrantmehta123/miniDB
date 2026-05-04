# TASK-015 — Query Plan: AST → Processor Pipeline

## Description
Replace the flat `execute_select` with a pull-based processor pipeline built from the AST.
Each processor is a node implementing a uniform `Processor` trait. This is foundational —
granule skipping (TASK-005), parallelism (TASK-006/007), and aggregation (TASK-008) all
wire into the pipeline rather than the flat function.

**Priority: do before TASK-005.**

---

## Design Decisions

- **Module**: `src/processors/` (not `planner/` — that scope includes optimizers, which are future work)
- **Model**: pull-based (Volcano/iterator): root processor drives execution by calling `next_batch()` on its child
- **Batch**: `Batch { schema: Vec<ColumnDef>, columns: Vec<ColumnChunk>, selection: Option<Vec<bool>> }`
  - Owned and moved through the chain — never cloned
  - `selection` is a lazy filter mask set by `Filter`; `None` means all rows are live
  - `Projection` is the only processor that physically compacts rows and drops columns
  - No partial-batch problem: `Filter` never changes row count, it only sets the mask
- **FullScan granularity**: one part per `next_batch()` call (granule-level batching deferred to TASK-005)
- **scan_cols**: `FullScan` reads `projection_cols ∪ predicate_cols` — the minimal set needed by the whole pipeline

---

## Steps

- [X] **`batch.rs`** — define `Batch` struct with `schema`, `columns`, `selection` fields

- [X] **`processor.rs`** — define `Processor` trait and `ExecutionError`
  - `fn next_batch(&mut self) -> Option<Result<Batch, ExecutionError>>`
  - `Option` signals exhaustion; `Result` signals a runtime fault (I/O, eval)

- [X] **`full_scan.rs`** — implement `FullScan`
  - Constructor: `FullScan::new(table_dir: PathBuf, columns: Vec<ColumnDef>)`
  - Discovers all parts at construction time; yields one `Batch` per part on each `next_batch()` call
  - `selection: None` on every emitted `Batch`

- [X] **`filter.rs`** — implement `Filter`
  - Constructor: `Filter::new(input: Box<dyn Processor>, predicate: Predicate)`
  - Calls `input.next_batch()`, evaluates predicate using the existing `evaluator::evaluate()`, sets `batch.selection`
  - Moves the `Batch` forward — no column data is copied

- [X] **`projection.rs`** — implement `Projection`
  - Constructor: `Projection::new(input: Box<dyn Processor>, output_cols: Vec<String>)`
  - Calls `input.next_batch()`, applies `selection` mask, keeps only `output_cols`, returns compacted `Batch`
  - This is the only processor that allocates new column data

- [X] **`mod.rs`** — implement `build_plan`
  ```
  1. scan_cols = projection_cols ∪ predicate_cols
  2. node = FullScan::new(table_dir, scan_cols)
  3. if WHERE clause:  node = Filter::new(node, predicate)
  4. if projection != *:  node = Projection::new(node, output_col_names)
  5. return node
  ```

- [X] **`executor.rs`** — replace `execute_select` body
  - Call `analyse_select` (validation unchanged), then `build_plan`, then drain `next_batch()` in a loop
  - Accumulate and merge `Batch` columns across calls; return `Vec<ColumnChunk>` as before

- [X] **`lib.rs`** — add `pub mod processors;`

- [X] **Delete dead code**
  - Flat predicate-wiring logic in `execute_select`
  - `ScanPlan` in `analyser.rs` — the planner owns `scan_cols` logic now

- [X] **All existing tests must still pass**

---

## Out of Scope
- Two-pass FullScan: read filter cols → get selection → read output cols for surviving rows only (deferred; this is where late materialization's real I/O benefit comes from)
- Granule-level batching within a part (TASK-005)
- Parallel operators (TASK-006/007)
- Aggregation operators (TASK-008)
