//! Sum aggregate over numeric types.
//!
//! State and output share the input type T. Overflow uses default `+`
//! (panic in debug, wrap in release). A future enhancement could promote
//! narrow integer inputs to a wider accumulator.

use std::marker::PhantomData;
use std::ops::AddAssign;

pub struct Sum<T>(PhantomData<T>);

impl<T> Sum<T>
where
    T: Copy + Default + AddAssign + std::iter::Sum,
{
    pub fn init() -> T {
        T::default()
    }

    pub fn update(state: &mut T, input: &[T]) {
        *state += input.iter().copied().sum::<T>();
    }

    pub fn merge(a: &mut T, b: T) {
        *a += b;
    }

    pub fn finalize(state: T) -> T {
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sum_i64_single_chunk() {
        let mut state = Sum::<i64>::init();
        Sum::<i64>::update(&mut state, &[1, 2, 3, 4]);
        assert_eq!(Sum::<i64>::finalize(state), 10);
    }

    #[test]
    fn sum_i32_single_chunk() {
        let mut state = Sum::<i32>::init();
        Sum::<i32>::update(&mut state, &[1, 2, 3, 4]);
        assert_eq!(Sum::<i32>::finalize(state), 10);
    }

    #[test]
    fn sum_empty_is_zero() {
        let mut state = Sum::<i64>::init();
        Sum::<i64>::update(&mut state, &[]);
        assert_eq!(Sum::<i64>::finalize(state), 0);
    }

    #[test]
    fn sum_i64_merge_matches_full() {
        let data: Vec<i64> = (1..=100).collect();

        let full = {
            let mut s = Sum::<i64>::init();
            Sum::<i64>::update(&mut s, &data);
            Sum::<i64>::finalize(s)
        };

        for split in [0, 1, 50, 99, 100] {
            let mut left = Sum::<i64>::init();
            Sum::<i64>::update(&mut left, &data[..split]);
            let mut right = Sum::<i64>::init();
            Sum::<i64>::update(&mut right, &data[split..]);
            Sum::<i64>::merge(&mut left, right);
            assert_eq!(Sum::<i64>::finalize(left), full, "split {split}");
        }
    }

    #[test]
    fn sum_i32_merge_matches_full() {
        let data: Vec<i32> = (1..=100).collect();

        let full = {
            let mut s = Sum::<i32>::init();
            Sum::<i32>::update(&mut s, &data);
            Sum::<i32>::finalize(s)
        };

        for split in [0, 1, 50, 99, 100] {
            let mut left = Sum::<i32>::init();
            Sum::<i32>::update(&mut left, &data[..split]);
            let mut right = Sum::<i32>::init();
            Sum::<i32>::update(&mut right, &data[split..]);
            Sum::<i32>::merge(&mut left, right);
            assert_eq!(Sum::<i32>::finalize(left), full, "split {split}");
        }
    }
}

use crate::aggregator::Aggregator;
use crate::processors::processor::ExecutionError;
use crate::storage::column_chunk::ColumnChunk;
use crate::storage::schema::DataType;

/// Runtime-dispatched wrapper around `Sum<T>`.
///
/// Holds a typed state matching the input column's `DataType`. Each call to
/// `update` matches both state and chunk to delegate to the right `Sum<T>`.
/// Output type equals input type (no widening yet).
enum SumState {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
}

pub struct SumAgg {
    state: SumState,
}

impl SumAgg {
    pub fn new(input: DataType) -> Result<Self, ExecutionError> {
        let state = match input {
            DataType::I8 => SumState::I8(Sum::<i8>::init()),
            DataType::I16 => SumState::I16(Sum::<i16>::init()),
            DataType::I32 => SumState::I32(Sum::<i32>::init()),
            DataType::I64 => SumState::I64(Sum::<i64>::init()),
            DataType::U8 => SumState::U8(Sum::<u8>::init()),
            DataType::U16 => SumState::U16(Sum::<u16>::init()),
            DataType::U32 => SumState::U32(Sum::<u32>::init()),
            DataType::U64 => SumState::U64(Sum::<u64>::init()),
            DataType::F32 => SumState::F32(Sum::<f32>::init()),
            DataType::F64 => SumState::F64(Sum::<f64>::init()),
            other => {
                return Err(ExecutionError::InvalidData(format!(
                    "SUM is not supported for type {:?}",
                    other
                )));
            }
        };
        Ok(Self { state })
    }
}

impl Aggregator for SumAgg {
    fn update(&mut self, chunk: &ColumnChunk) -> Result<(), ExecutionError> {
        match (&mut self.state, chunk) {
            (SumState::I8(s), ColumnChunk::I8(v)) => Sum::<i8>::update(s, v),
            (SumState::I16(s), ColumnChunk::I16(v)) => Sum::<i16>::update(s, v),
            (SumState::I32(s), ColumnChunk::I32(v)) => Sum::<i32>::update(s, v),
            (SumState::I64(s), ColumnChunk::I64(v)) => Sum::<i64>::update(s, v),
            (SumState::U8(s), ColumnChunk::U8(v)) => Sum::<u8>::update(s, v),
            (SumState::U16(s), ColumnChunk::U16(v)) => Sum::<u16>::update(s, v),
            (SumState::U32(s), ColumnChunk::U32(v)) => Sum::<u32>::update(s, v),
            (SumState::U64(s), ColumnChunk::U64(v)) => Sum::<u64>::update(s, v),
            (SumState::F32(s), ColumnChunk::F32(v)) => Sum::<f32>::update(s, v),
            (SumState::F64(s), ColumnChunk::F64(v)) => Sum::<f64>::update(s, v),
            _ => {
                return Err(ExecutionError::InvalidData(
                    "SUM: state/chunk type mismatch (planner bug)".into(),
                ));
            }
        }
        Ok(())
    }

    fn finalize(&mut self) -> ColumnChunk {
        match self.state {
            SumState::I8(s) => ColumnChunk::I8(vec![Sum::<i8>::finalize(s)]),
            SumState::I16(s) => ColumnChunk::I16(vec![Sum::<i16>::finalize(s)]),
            SumState::I32(s) => ColumnChunk::I32(vec![Sum::<i32>::finalize(s)]),
            SumState::I64(s) => ColumnChunk::I64(vec![Sum::<i64>::finalize(s)]),
            SumState::U8(s) => ColumnChunk::U8(vec![Sum::<u8>::finalize(s)]),
            SumState::U16(s) => ColumnChunk::U16(vec![Sum::<u16>::finalize(s)]),
            SumState::U32(s) => ColumnChunk::U32(vec![Sum::<u32>::finalize(s)]),
            SumState::U64(s) => ColumnChunk::U64(vec![Sum::<u64>::finalize(s)]),
            SumState::F32(s) => ColumnChunk::F32(vec![Sum::<f32>::finalize(s)]),
            SumState::F64(s) => ColumnChunk::F64(vec![Sum::<f64>::finalize(s)]),
        }
    }

    fn output_type(&self) -> DataType {
        match self.state {
            SumState::I8(_) => DataType::I8,
            SumState::I16(_) => DataType::I16,
            SumState::I32(_) => DataType::I32,
            SumState::I64(_) => DataType::I64,
            SumState::U8(_) => DataType::U8,
            SumState::U16(_) => DataType::U16,
            SumState::U32(_) => DataType::U32,
            SumState::U64(_) => DataType::U64,
            SumState::F32(_) => DataType::F32,
            SumState::F64(_) => DataType::F64,
        }
    }
}
