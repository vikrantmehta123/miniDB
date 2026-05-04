# Deferred Tasks

These tasks are not definitively in Phase 1 or Phase 2. But
these tasks naturally came up during implementations of the 
currently scoped tasks and could be worth thinking over as 
future additions to the spec.

1. Zero-copy buffering using bytemuck

2. Adaptive Granularity: Currently each granule is fixed at
512-row granules for column types. For long strings, this can
get too big. So ClickHouse implements Adaptive Granularity,
which could be worth expploring. 

3. Delta-Decode and Dictionary Lookups are SIMD paths. So potentially this could be implemented.

4. Operator-based execution engine — streaming, query plan, parallel scan.
   The flat `execute_select` pipeline works for correctness but has three
   known limitations to address together:

   - **Query plan construction**: the analyser should produce a `PhysicalPlan`
     tree (`Scan`, `Filter`, `Project`, ...) and the executor should walk it
     generically via a `PhysicalOperator::next() -> Result<Option<Batch>>`
     trait, rather than hard-coding the pipeline in `execute_select`.

   - **Memory-bounded output**: `read_all()` materializes the full dataset.
     Once the scan operator exists, it should emit one granule at a time so
     callers can stream results without holding the whole table in memory.

   - **Parallel part reads**: parts are independent; `rayon::par_iter()` over
     part IDs in the scan operator is a natural fit once the operator owns its
     own iteration loop.