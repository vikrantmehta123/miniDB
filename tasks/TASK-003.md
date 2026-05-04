# TASK-003 — WHERE: Parser Lowering

## Description
Extend the SQL parser layer to carry an optional `Predicate` tree from the sqlparser-rs AST into the internal `Statement::Select`. This is the first of three WHERE tasks. No evaluation yet — just the AST representation and lowering.

**Sprint 2 — estimated 1 session.**

---

## Steps

- [ ] **Define `Predicate` enum** (`src/parser/ast.rs`)
  - `Predicate::Cmp { col: String, op: CmpOp, value: Literal }` for leaf predicates
  - `Predicate::And(Box<Predicate>, Box<Predicate>)`
  - `Predicate::Or(Box<Predicate>, Box<Predicate>)`
  - `Predicate::Not(Box<Predicate>)`
  - `CmpOp` enum: `Eq, Ne, Lt, Le, Gt, Ge`
  - `Literal` enum: `Int(i64), Float(f64), Str(String), Bool(bool)`

- [ ] **Lower WHERE clause** (`src/parser/lower.rs`)
  - Walk the sqlparser-rs `Expr` for the WHERE clause recursively
  - Map `BinaryOp` with `And/Or` → `Predicate::And/Or`
  - Map `UnaryOp(Not, ...)` → `Predicate::Not`
  - Map `BinaryOp` with comparison ops → `Predicate::Cmp`
  - Return `Err` with a clear message for subqueries, functions, IS NULL, BETWEEN, IN

- [ ] **Carry predicate in `Statement::Select`**
  - Add `where_clause: Option<Predicate>` to the `Select` struct
  - Executor receives it (can ignore for now — evaluation is TASK-004)

- [ ] **Test**: parse `WHERE ts > 1000 AND uid = 42`, assert the predicate tree structure

---

## Out of Scope
- Predicate evaluation (TASK-004)
- Granule skipping (TASK-005)
- LIKE, IN, BETWEEN, IS NULL
