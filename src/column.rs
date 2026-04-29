use crate::types::IDataType;

pub trait IColumn {
    fn len(&self) -> usize;
    fn serialize_binary_bulk(&self, buf: &mut Vec<u8>, offset: usize, limit: usize);
    fn deserialize_binary_bulk(&mut self, buf: &[u8]);
}

pub struct ColumnVector<T: IDataType> {
    pub data: Vec<T>,
}

impl<T: IDataType> IColumn for ColumnVector<T> {
    fn len(&self) -> usize { 
        self.data.len() 
    }

    fn serialize_binary_bulk(&self, buf: &mut Vec<u8>, offset: usize, limit: usize) {
        for value in &self.data[offset..offset+limit] {
           buf.extend(value.to_le_bytes_vec()); 
        }
    }

    fn deserialize_binary_bulk(&mut self, buf: &[u8]) {
        for chunk in buf.chunks_exact(T::size_of()) {
            self.data.push(T::from_le_bytes(chunk));
        }
    }
}

