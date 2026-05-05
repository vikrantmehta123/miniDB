//! Avg aggregate over numeric types.
//!
//! Always accumulates in f64 and returns f64 regardless of input type —
//! casting integers up avoids both overflow and a per-type state enum.
//! No generic Avg<T> math struct: the accumulation is trivial enough that
//! splitting it out would add indirection for no gain.

use crate::aggregator::Aggregator;
use crate::processors::processor::ExecutionError;
use crate::storage::column_chunk::ColumnChunk;
use crate::storage::schema::DataType;

pub struct AvgAgg {
    sum:   f64,
    count: u64,
}

impl AvgAgg {
    pub fn new(input: DataType) -> Result<Self, ExecutionError> {
        match input {
            DataType::Bool | DataType::Str => Err(ExecutionError::InvalidData(
                format!("AVG is not supported for type {:?}", input)
            )),
            _ => Ok(Self { sum: 0.0, count: 0 }),
        }
    }
}

impl Aggregator for AvgAgg {
    fn update(&mut self, chunk: &ColumnChunk) -> Result<(), ExecutionError> {
        match chunk {
            ColumnChunk::I8(v)  => { self.sum += v.iter().map(|&x| x as f64).sum::<f64>(); self.count += v.len() as u64; }
            ColumnChunk::I16(v) => { self.sum += v.iter().map(|&x| x as f64).sum::<f64>(); self.count += v.len() as u64; }
            ColumnChunk::I32(v) => { self.sum += v.iter().map(|&x| x as f64).sum::<f64>(); self.count += v.len() as u64; }
            ColumnChunk::I64(v) => { self.sum += v.iter().map(|&x| x as f64).sum::<f64>(); self.count += v.len() as u64; }
            ColumnChunk::U8(v)  => { self.sum += v.iter().map(|&x| x as f64).sum::<f64>(); self.count += v.len() as u64; }
            ColumnChunk::U16(v) => { self.sum += v.iter().map(|&x| x as f64).sum::<f64>(); self.count += v.len() as u64; }
            ColumnChunk::U32(v) => { self.sum += v.iter().map(|&x| x as f64).sum::<f64>(); self.count += v.len() as u64; }
            ColumnChunk::U64(v) => { self.sum += v.iter().map(|&x| x as f64).sum::<f64>(); self.count += v.len() as u64; }
            ColumnChunk::F32(v) => { self.sum += v.iter().map(|&x| x as f64).sum::<f64>(); self.count += v.len() as u64; }
            ColumnChunk::F64(v) => { self.sum += v.iter().copied().sum::<f64>();             self.count += v.len() as u64; }
            _ => return Err(ExecutionError::InvalidData(
                "AVG: unsupported column type (planner bug)".into()
            )),
        }
        Ok(())
    }

    fn finalize(&mut self) -> ColumnChunk {
        let result = if self.count == 0 { 0.0 } else { self.sum / self.count as f64 };
        ColumnChunk::F64(vec![result])
    }

    fn output_type(&self) -> DataType {
        DataType::F64
    }
}
