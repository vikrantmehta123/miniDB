# tinyOLAP — Benchmark Results

**Date:** 2026-05-06  
**Machine:** Intel Core i5-10210U @ 1.60GHz (4 cores / 8 threads, max 4.2 GHz), Linux 6.8.0  
**Build:** `cargo bench` (release profile, LTO default)  
**Data size:** 1 000 000 rows unless noted

---

## Storage Write — `cargo bench --bench storage_write`

Schema variants:
- **single_i64** — `ts: i64` (1 column, sort key)
- **wide_numeric_5col** — `ts: i64, uid: u32, val: f64, flags: u8, score: f32`
- **mixed_4col** — `ts: i64, uid: u32, event: Str, val: f64`

| Benchmark | Median time | Throughput |
|---|---|---|
| single_i64_1M | 16.2 ms | 472 MiB/s |
| wide_numeric_5col_1M | 104.8 ms | 227 MiB/s |
| mixed_4col_1M | 214.7 ms | 115 MiB/s |

**Takeaways:**
- Wide numeric is 2.5× slower per raw MB than single column — the sort permutation touches all 5 column vectors simultaneously, thrashing cache.
- Mixed (strings + numeric) is another 2× slower on top — string encoding has no fixed stride and can't vectorize.

---

## Storage Scan — `cargo bench --bench storage_scan`

| Benchmark | Median time | Throughput |
|---|---|---|
| single_col_pruned_1M | 11.1 ms | 689 MiB/s |
| all_cols_5col_1M | 53.5 ms | 445 MiB/s |
| string_col_1M | 132.4 ms | 42 MiB/s |
| multipart_i64 / 10 × 100k rows | 5.6 ms | 1.33 GiB/s |
| multipart_i64 / 100 × 10k rows | 4.9 ms | 1.50 GiB/s |

**Takeaways:**
- Column pruning is linear: 1 column takes 11 ms, 5 columns take 53 ms (~5× proportional). Reads are independent per column, no cross-column overhead.
- String decode is 16× slower than numeric (42 vs 689 MiB/s). The bottleneck is 1M heap `String` allocations in the decode loop, not I/O.
- Parallelism is at the part level (Rayon). More parts → more Rayon tasks → better core saturation. 100 parts achieves ~2× the throughput of a single large part.
- No parallel column reads within a part — columns are decoded sequentially. This is where future work will help.

---

## Encoding Codecs — `cargo bench --bench encoding_codecs`

All benchmarks: 1M `i64` values (8 MB), pure in-memory (no disk I/O).

### Plain

| Benchmark | Median time | Throughput |
|---|---|---|
| encode_i64_1M | 673 µs | 11.1 GiB/s |
| decode_i64_1M | 686 µs | 10.9 GiB/s |

Near-memcpy speed. This is the ceiling for all other codecs.

### Delta

| Benchmark | Median time | Throughput |
|---|---|---|
| encode_i64_1M / sorted | 3.30 ms | 2.26 GiB/s |
| decode_i64_1M / sorted | 3.47 ms | 2.15 GiB/s |
| encode_i64_1M / random | 3.53 ms | 2.11 GiB/s |
| decode_i64_1M / random | 3.53 ms | 2.09 GiB/s |

Sorted and random patterns are nearly identical (~7% apart). Delta does the same wrapping arithmetic regardless of data pattern — the pattern only affects how well the *output* compresses with LZ4. Delta is ~5× slower than plain.

### RLE

| Benchmark | Median time | Throughput |
|---|---|---|
| encode_i64_1M / high_run | 5.1 ms | 1.45 GiB/s |
| decode_i64_1M / high_run | 4.5 ms | 1.66 GiB/s |
| encode_i64_1M / low_cardinality | 10.1 ms | 753 MiB/s |
| decode_i64_1M / low_cardinality | 6.2 ms | 1.21 GiB/s |
| encode_i64_1M / all_unique | 7.8 ms | 977 MiB/s |
| decode_i64_1M / all_unique | 6.3 ms | 1.18 GiB/s |

- `high_run` (all same value) is fastest to encode — inner loop just increments a counter, branch-predictor friendly.
- `low_cardinality` uses values cycling `0,1,2,...9,0,1,2...` — consecutive values are always different, so every run has length 1. RLE *expands* the data (10 bytes output per 8 bytes input). Slower than all_unique due to the cycling write pattern.
- Decode throughput is nearly the same for low_cardinality and all_unique — decode always does the same work per output element regardless of run length.
- RLE only pays off when runs are long (booleans, status flags, low-cardinality enums).

### String Codecs

| Benchmark | Median time | Throughput |
|---|---|---|
| encode_1M / plain | 7.3 ms | 760 MiB/s |
| decode_1M / plain | 112 ms | 49 MiB/s |
| encode_1M / dictionary | 80 ms | 69 MiB/s |
| decode_1M / dictionary | 106 ms | 52 MiB/s |

- Plain encode is 15× faster than plain decode. Encode iterates `&[String]` and copies bytes — no allocations. Decode creates 1M `String` objects — heap allocation dominates everything.
- Dictionary encode is 11× slower than plain encode: building the `HashMap<string → index>` over 1M entries costs more than a raw byte copy.
- Dictionary decode ≈ plain decode (~50 MiB/s). Both are heap-allocation bound; the codec choice is irrelevant until string materialisation is addressed.
- Root cause: `LowCardinality(String)` (Phase 2) will fix this by storing integer indices internally and only allocating on final output.

---

## Query Pipeline — `cargo bench --bench query_pipeline`

Schema: `ts: i64, uid: u32, event: Str, val: f64`. Single part, 1M rows.

| Benchmark | Median time | Throughput |
|---|---|---|
| full_scan_no_filter | 55.4 ms | 18.0 Melem/s |
| filter_selectivity / 1% | 48.5 ms | 20.6 Melem/s |
| filter_selectivity / 10% | 48.6 ms | 20.6 Melem/s |
| filter_selectivity / 50% | 55.4 ms | 18.0 Melem/s |
| aggregate_no_group_by | 20.4 ms | 49.0 Melem/s |
| group_by / low 10 groups | 572.6 ms | 1.75 Melem/s |
| group_by / high 100k groups | 553.0 ms | 1.81 Melem/s |

**Takeaways:**

- **Filter selectivity is flat** — 1% and 10% are only marginally faster than no_filter, and only because they project 2 columns instead of 3. Without an index, all rows are read regardless; selectivity only saves on output materialisation.

- **Aggregate beats full scan** — `SUM(val), COUNT(*), AVG(val)` reads only the `val` column (1 of 4). Output is 3 scalars. 20ms vs 55ms is the columnar projection win.

- **GROUP BY is 10× slower than aggregate** — both low and high cardinality take ~560ms vs 20ms for a plain aggregate. The hash map over 1M rows shouldn't cost 540ms extra; likely excessive per-row cloning in `group_by_aggregate.rs`. Priority investigation target.

- **Low cardinality GROUP BY ≈ high cardinality GROUP BY** — counterintuitive. Low (10 groups) uses the `event` string column as key, paying string hashing cost for 1M rows. High (100k groups) uses `uid: u32`, which is cheap to hash but causes cache misses across a 100k-entry map. Both bottlenecks land at ~550ms.

---

## What These Numbers Point To

| Area | Current bottleneck | Future fix |
|---|---|---|
| String decode | 1M heap allocations | `LowCardinality(String)` (Phase 2) |
| Numeric encode/decode | Sequential, no SIMD | `std::simd` vectorisation (Phase 1) |
| GROUP BY | Likely per-row string cloning | Profile + fix `group_by_aggregate.rs` |
| Single-part scan | Sequential column reads | Granule-level parallelism (Phase 1) |
