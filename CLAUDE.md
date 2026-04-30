# miniDB — Columnar Database in Rust

## Project Goals
1. **Learn Rust by doing** — concurrency, ownership, safety, traits, async
2. **Build a columnar database from scratch** — understand how real column stores work

## Core Design Decisions
- **Columnar storage**: data stored column-by-column, not row-by-row
- **Chunk size**: 1024 values per chunk (the atomic unit of processing)
- **Supported types**: all numeric types (`i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, `f64`), `bool`, variable-length strings
- **Disk-persistent**: data lives on disk, not in memory — no in-memory-only database
- **No custom parser**: use an off-the-shelf SQL parser crate
- **Features evolve as we write code** — don't over-plan

## What to Emphasize as We Build
- Use Rust idioms: `Result`/`Option` over panics, iterators over loops where natural
- Prefer safe code; use `unsafe` only when necessary and document why
- Leverage Rust's type system to model the column type system (e.g., enums + generics)
- Introduce concurrency (e.g., `rayon`, channels, `Arc<Mutex<>>`) when it fits naturally
- Keep it simple — no premature abstractions

## Current Module Structure
```
src/
  main.rs           # CLI entry point — dispatches to run::<T>() for numerics, run_string() for strings
  config.rs         # GRANULE_SIZE (512) and BLOCK_BUFFER_SIZE (8KB) constants
  data_type.rs      # IDataType trait + impls for all 10 numeric types (i8/i16/i32/i64/u8/u16/u32/u64/f32/f64)
  column.rs         # IColumn trait + generic ColumnVector<T: IDataType>
  storage.rs        # ColumnWriter<T> (push/flush) and ColumnReader (read_granule/read_all) — granule/LZ4-compress pipeline with single-block cache
  mark.rs           # Mark struct, MarkWriter (buffered), MarkReader
  string_column.rs  # StringColumn, StringColumnWriter (push/flush) and StringColumnReader (read_granule/read_all) — same pipeline, dual granule boundary (count + byte size)
```

Future modules (not yet started):
```
  catalog/      # Table and schema metadata
  executor/     # Query execution (vectorized, chunk-at-a-time)
  parser/       # Thin wrapper around the chosen SQL parser crate
```

## Current Task
- Active work is tracked in `TASK.md` at the repo root. Always read it at the start of a session to know what we're building next and which step we're on.
- Feature scope and phasing is in `SPEC.md` at the repo root. Read it for context on what Phase 1 covers and what is deferred to Phase 2.

## Current Progress
- **Variable-length string column fully implemented and round-trip verified.**
- `src/config.rs`: `GRANULE_SIZE = 512`, `BLOCK_BUFFER_SIZE = 8KB` — shared by numeric and string storage.
- `src/data_type.rs`: `IDataType` trait with `name()`, `size_of()`, `to_le_bytes_vec()`, `from_le_bytes()` — implemented for all numeric primitives.
- `src/column.rs`: `IColumn` trait (`len`, `serialize_binary_bulk`, `deserialize_binary_bulk`) + `ColumnVector<T: IDataType>`.
- `src/mark.rs`: `Mark` struct with `to_bytes()`/`from_bytes()`; `MarkWriter` (buffers all marks in memory, single `flush()` write); `MarkReader` (`read_all()` reads entire `.mrk` file at once).
- `src/storage.rs`: `ColumnWriter<T>` (`push(val: T)`, `flush()` — granule loop → LZ4 compress → write block); `ColumnReader` (no struct-level generic; `read_granule<T>` with single-block decompression cache keyed on `block_offset`, `read_all<T>`).
- `src/string_column.rs`: `StringColumn { data: Vec<String> }`; `StringColumnWriter` (`push`, `flush` — dual granule boundary: 512 strings or 8KB, whichever comes first; strings never split across blocks); `StringColumnReader` (`read_granule` → `StringColumn`, single-block cache, sequential `i32`-prefixed deserialization; `read_all`). `String` does not implement `IColumn` — `StringColumn` is a parallel but separate type.
- `src/main.rs`: `run::<T>()` for all 10 numeric types; `run_string()` for strings — both assert round-trip correctness.

## Build System
- Standard `cargo` — `cargo build`, `cargo run`, `cargo test`
- No special flags needed beyond what Cargo.toml specifies

## Style
- Move fast, keep it simple — this is a learning project, not production software
- Prefer clarity over cleverness unless the clever version teaches something about Rust
- Tests are good when they lock in correctness of a tricky piece; don't test everything

## Collaboration Rules
- **Claude must NOT write code to files.** The user types every line themselves.
- Claude's role: explain concepts, show code snippets in chat, guide decisions, answer questions.
- Exceptions: documentation/docstrings only (e.g., updating CLAUDE.md).
