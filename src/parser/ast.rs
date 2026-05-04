#[derive(Debug, Clone)]
pub enum Statement {
    Insert(InsertStmt),
    Select(SelectStmt),
}

#[derive(Debug, Clone)]
pub struct InsertStmt {
    pub table: String,
    pub columns: Option<Vec<String>>,  // None = all schema columns in order
    pub rows: Vec<Vec<Literal>>,       // batch — outer Vec is rows
}

#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64),
    UInt(u64),
    Float(f64),
    Bool(bool),
    Str(String),
    Null,
}

#[derive(Debug, Clone)]
pub enum Projection {
    All,
    Columns(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct SelectStmt {
    pub table: String,
    pub projection: Projection,
}
