# miniDB — Columnar Database in Rust

## Project Goals
1. **Learn Rust by doing** — concurrency, ownership, safety, traits, async
2. **Build a columnar database from scratch** — understand how real column stores work

## Core Design Decisions
- **Columnar storage**: data stored column-by-column, not row-by-row
- **Chunk size**: 1024 values per chunk (the atomic unit of processing)
- **Supported types**: all numeric types (`i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, `f64`), `bool`, fixed-size strings
- **Disk-persistent**: data lives on disk, not in memory — no in-memory-only database
- **No custom parser**: use an off-the-shelf SQL parser crate
- **Features evolve as we write code** — don't over-plan

## What to Emphasize as We Build
- Use Rust idioms: `Result`/`Option` over panics, iterators over loops where natural
- Prefer safe code; use `unsafe` only when necessary and document why
- Leverage Rust's type system to model the column type system (e.g., enums + generics)
- Introduce concurrency (e.g., `rayon`, channels, `Arc<Mutex<>>`) when it fits naturally
- Keep it simple — no premature abstractions

## Likely Module Structure (evolves over time)
```
src/
  main.rs          # REPL / entry point
  types.rs         # DataType enum, Value enum
  column.rs        # Column and Chunk abstractions
  storage/         # How chunks are stored on disk
  catalog/         # Table and schema metadata
  executor/        # Query execution (vectorized, chunk-at-a-time)
  parser/          # Thin wrapper around the chosen SQL parser crate
```

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
