use super::batch::Batch;

#[derive(Debug)]
pub enum ExecutionError {
    Io(std::io::Error),
    InvalidData(String),
}

impl From<std::io::Error> for ExecutionError {
    fn from(e: std::io::Error) -> Self {
        ExecutionError::Io(e)
    }
}

pub trait Processor {
    fn next_batch(&mut self) -> Option<Result<Batch, ExecutionError>>;
}
