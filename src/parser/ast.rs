#[derive(Debug, Clone)]
pub enum Statement {
    Insert(InsertStmt),
    Select(SelectStmt),
}

#[derive(Debug, Clone)]
pub struct InsertStmt {
    pub table: String,
    pub columns: Option<Vec<String>>,
    pub rows: Vec<Vec<Literal>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    UInt(u64),
    Float(f64),
    Bool(bool),
    Str(String),
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AggFunc {
    Sum,
    Max,
    Min,
    Count,
    Avg,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectExpr {
    Col(String),
    Agg { func: AggFunc, col: String },
}

#[derive(Debug, Clone)]
pub enum Projection {
    All,
    Exprs(Vec<SelectExpr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CmpOp {
    Eq, Ne, Lt, Le, Gt, Ge,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Predicate {
    Cmp { col: String, op: CmpOp, value: Literal },
    And(Box<Predicate>, Box<Predicate>),
    Or(Box<Predicate>, Box<Predicate>),
    Not(Box<Predicate>),
}

#[derive(Debug, Clone)]
pub struct SelectStmt {
    pub table: String,
    pub projection: Projection,
    pub where_clause: Option<Predicate>,
}
