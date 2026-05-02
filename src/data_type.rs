

pub trait IDataType: Sized + Copy {
    fn name() -> &'static str;
    fn size_of() -> usize;
    fn extend_le_bytes(&self, out: &mut Vec<u8>);
    fn from_le_bytes(bytes: &[u8]) -> Self;
}

impl IDataType for i64 {
    fn name() -> &'static str { "Int64" }
    fn size_of() -> usize { 8 }
    fn extend_le_bytes(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }

    fn from_le_bytes(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().unwrap())
    }
}

impl IDataType for f64 {
    fn name() -> &'static str { "Float64" }
    fn size_of() -> usize { 8 } 
    fn extend_le_bytes(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }   
    fn from_le_bytes(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().unwrap())
    } 
    
}


impl IDataType for f32 {
    fn name() -> &'static str { "Float32" }
    fn size_of() -> usize { 4 } 
    fn extend_le_bytes(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }    

    fn from_le_bytes(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().unwrap())
    }
}


impl IDataType for i32 {
    fn name() -> &'static str { "Int32" }
    fn size_of() -> usize { 4 }
    fn extend_le_bytes(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    } 
    fn from_le_bytes(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().unwrap())
    }
}


impl IDataType for i16 {
    fn name() -> &'static str { "Int16" }
    fn size_of() -> usize { 2 }
    fn extend_le_bytes(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    } 
    fn from_le_bytes(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().unwrap())
    }
}


impl IDataType for i8 {
    fn name() -> &'static str { "Int8" }
    fn size_of() -> usize { 1 }
    fn extend_le_bytes(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    } 
    fn from_le_bytes(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().unwrap())
    }
}


impl IDataType for u8 {
    fn name() -> &'static str { "UInt8" }
    fn size_of() -> usize { 1 }
    fn extend_le_bytes(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    } 
    fn from_le_bytes(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().unwrap())
    }
}


impl IDataType for u16 {
    fn name() -> &'static str { "UInt16" }
    fn size_of() -> usize { 2 }
    fn extend_le_bytes(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    } 
    fn from_le_bytes(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().unwrap())
    }
}


impl IDataType for u32 {
    fn name() -> &'static str { "UInt32" }
    fn size_of() -> usize { 4 }
    fn extend_le_bytes(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    } 
    fn from_le_bytes(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().unwrap())
    }
}


impl IDataType for u64 {
    fn name() -> &'static str { "UInt64" }
    fn size_of() -> usize { 8 }
    fn extend_le_bytes(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    } 
    fn from_le_bytes(bytes: &[u8]) -> Self {
        Self::from_le_bytes(bytes.try_into().unwrap())
    }
}


impl IDataType for bool {
    fn name() -> &'static str { "Bool" }
    fn size_of() -> usize { 1 } 
    fn extend_le_bytes(&self, out: &mut Vec<u8>) {
        out.push(*self as u8);
    } 
    fn from_le_bytes(bytes: &[u8]) -> Self {
        bytes[0] != 0
    }
}

