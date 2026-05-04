use crate::storage::schema::DataType;

#[derive(Debug)]
pub enum ColumnChunk {
    I8(Vec<i8>),
    I16(Vec<i16>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    U8(Vec<u8>),
    U16(Vec<u16>),
    U32(Vec<u32>),
    U64(Vec<u64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    Bool(Vec<bool>),
    Str(Vec<String>),
}

impl ColumnChunk {
    pub fn len(&self) -> usize {
        match self {
            ColumnChunk::I8(v) => v.len(),
            ColumnChunk::I16(v) => v.len(),
            ColumnChunk::I32(v) => v.len(),
            ColumnChunk::I64(v) => v.len(),
            ColumnChunk::U8(v) => v.len(),
            ColumnChunk::U16(v) => v.len(),
            ColumnChunk::U32(v) => v.len(),
            ColumnChunk::U64(v) => v.len(),
            ColumnChunk::F32(v) => v.len(),
            ColumnChunk::F64(v) => v.len(),
            ColumnChunk::Bool(v) => v.len(),
            ColumnChunk::Str(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn filter(&self, mask: &[bool]) -> ColumnChunk {
        match self {
            ColumnChunk::I8(v) => ColumnChunk::I8(
                v.iter()
                    .zip(mask)
                    .filter_map(|(&x, &m)| m.then_some(x))
                    .collect(),
            ),
            ColumnChunk::I16(v) => ColumnChunk::I16(
                v.iter()
                    .zip(mask)
                    .filter_map(|(&x, &m)| m.then_some(x))
                    .collect(),
            ),
            ColumnChunk::I32(v) => ColumnChunk::I32(
                v.iter()
                    .zip(mask)
                    .filter_map(|(&x, &m)| m.then_some(x))
                    .collect(),
            ),
            ColumnChunk::I64(v) => ColumnChunk::I64(
                v.iter()
                    .zip(mask)
                    .filter_map(|(&x, &m)| m.then_some(x))
                    .collect(),
            ),
            ColumnChunk::U8(v) => ColumnChunk::U8(
                v.iter()
                    .zip(mask)
                    .filter_map(|(&x, &m)| m.then_some(x))
                    .collect(),
            ),
            ColumnChunk::U16(v) => ColumnChunk::U16(
                v.iter()
                    .zip(mask)
                    .filter_map(|(&x, &m)| m.then_some(x))
                    .collect(),
            ),
            ColumnChunk::U32(v) => ColumnChunk::U32(
                v.iter()
                    .zip(mask)
                    .filter_map(|(&x, &m)| m.then_some(x))
                    .collect(),
            ),
            ColumnChunk::U64(v) => ColumnChunk::U64(
                v.iter()
                    .zip(mask)
                    .filter_map(|(&x, &m)| m.then_some(x))
                    .collect(),
            ),
            ColumnChunk::F32(v) => ColumnChunk::F32(
                v.iter()
                    .zip(mask)
                    .filter_map(|(&x, &m)| m.then_some(x))
                    .collect(),
            ),
            ColumnChunk::F64(v) => ColumnChunk::F64(
                v.iter()
                    .zip(mask)
                    .filter_map(|(&x, &m)| m.then_some(x))
                    .collect(),
            ),
            ColumnChunk::Bool(v) => ColumnChunk::Bool(
                v.iter()
                    .zip(mask)
                    .filter_map(|(&x, &m)| m.then_some(x))
                    .collect(),
            ),
            ColumnChunk::Str(v) => ColumnChunk::Str(
                v.iter()
                    .zip(mask)
                    .filter_map(|(x, &m)| m.then_some(x.clone()))
                    .collect(),
            ),
        }
    }

    pub fn data_type(&self) -> DataType {
        match self {
            ColumnChunk::I8(_) => DataType::I8,
            ColumnChunk::I16(_) => DataType::I16,
            ColumnChunk::I32(_) => DataType::I32,
            ColumnChunk::I64(_) => DataType::I64,
            ColumnChunk::U8(_) => DataType::U8,
            ColumnChunk::U16(_) => DataType::U16,
            ColumnChunk::U32(_) => DataType::U32,
            ColumnChunk::U64(_) => DataType::U64,
            ColumnChunk::F32(_) => DataType::F32,
            ColumnChunk::F64(_) => DataType::F64,
            ColumnChunk::Bool(_) => DataType::Bool,
            ColumnChunk::Str(_) => DataType::Str, // not String — schema uses Str
        }
    }
}
