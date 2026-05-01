use std::path::{Path};
use std::fs;
use crate::schema::{TableDef, DataType, Value};
use crate::part::Part;
use crate::storage::ColumnWriter;
use crate::string_column::StringColumnWriter;


enum WriterBox {
    I8(ColumnWriter<i8>),
    I16(ColumnWriter<i16>),
    I32(ColumnWriter<i32>),
    I64(ColumnWriter<i64>),
    U8(ColumnWriter<u8>),
    U16(ColumnWriter<u16>),
    U32(ColumnWriter<u32>),
    U64(ColumnWriter<u64>),
    F32(ColumnWriter<f32>),
    F64(ColumnWriter<f64>),
    Bool(ColumnWriter<bool>),
    Str(StringColumnWriter),
}

impl WriterBox {
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            WriterBox::I8(w) => w.flush(),
            WriterBox::I16(w) => w.flush(),
            WriterBox::I32(w) => w.flush(),
            WriterBox::I64(w) => w.flush(),
            WriterBox::U8(w) => w.flush(),
            WriterBox::U16(w) => w.flush(), 
            WriterBox::U32(w) => w.flush(),
            WriterBox::U64(w) => w.flush(),
            WriterBox::F32(w) => w.flush(),
            WriterBox::F64(w) => w.flush(),
            WriterBox::Bool(w) => w.flush(),
            WriterBox::Str(w) => w.flush()      
        }
    }

    fn push_column(&mut self, vals: Vec<Value>) -> std::io::Result<()> {
        for val in vals {
            match (self, val) {
                (WriterBox::I8(w),   Value::I8(v))   => w.push(v),
                (WriterBox::I16(w),  Value::I16(v))  => w.push(v),
                (WriterBox::I32(w),  Value::I32(v))  => w.push(v),
                (WriterBox::I64(w),  Value::I64(v))  => w.push(v),
                (WriterBox::U8(w),   Value::U8(v))   => w.push(v),
                (WriterBox::U16(w),  Value::U16(v))  => w.push(v),
                (WriterBox::U32(w),  Value::U32(v))  => w.push(v),
                (WriterBox::U64(w),  Value::U64(v))  => w.push(v),
                (WriterBox::F32(w),  Value::F32(v))  => w.push(v),
                (WriterBox::F64(w),  Value::F64(v))  => w.push(v),
                (WriterBox::Bool(w), Value::Bool(v)) => w.push(v),
                (WriterBox::Str(w),  Value::Str(v))  => w.push(v),
                _ => return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "type mismatch")),
            }
        }
        Ok(())
    }

}


pub struct TableWriter {
    pub part: Part, 
    pub def: TableDef,
    writers: Vec<WriterBox>,
}

impl TableWriter {
    pub fn create(table_dir: &Path) -> std::io::Result<Self> {
        let def = TableDef::open(table_dir)?;
        let next_id = Self::next_part_id(table_dir)?;
        let part = Part::new(TableDef::part_dir(table_dir, next_id));
        part.create_dir()?;

        let mut writers = Vec::new();
        for col in &def.columns {
            let bin = part.column_bin_path(col);
            let mrk = part.column_mrk_path(col);
            let w = match col.data_type {
                DataType::I8   => WriterBox::I8(ColumnWriter::create(&bin, &mrk)?),
                DataType::I16  => WriterBox::I16(ColumnWriter::create(&bin, &mrk)?),
                DataType::I32  => WriterBox::I32(ColumnWriter::create(&bin, &mrk)?),
                DataType::I64  => WriterBox::I64(ColumnWriter::create(&bin, &mrk)?),
                DataType::U8   => WriterBox::U8(ColumnWriter::create(&bin, &mrk)?),
                DataType::U16  => WriterBox::U16(ColumnWriter::create(&bin, &mrk)?),
                DataType::U32  => WriterBox::U32(ColumnWriter::create(&bin, &mrk)?),
                DataType::U64  => WriterBox::U64(ColumnWriter::create(&bin, &mrk)?),
                DataType::F32  => WriterBox::F32(ColumnWriter::create(&bin, &mrk)?),
                DataType::F64  => WriterBox::F64(ColumnWriter::create(&bin, &mrk)?),
                DataType::Bool => WriterBox::Bool(ColumnWriter::create(&bin, &mrk)?),
                DataType::Str  => WriterBox::Str(StringColumnWriter::create(&bin, &mrk)?),
            };
            writers.push(w);
        }

        Ok(TableWriter {part, def, writers })
    }

    fn next_part_id(table_dir: &Path) -> std::io::Result<u32> {
        let mut max_id = 0u32;
        for entry in fs::read_dir(table_dir)? {
            let name = entry?.file_name();
            let s = name.to_string_lossy();
            if let Some(suffix) = s.strip_prefix("part_") {
                if let Ok(id) = suffix.parse::<u32>() {
                    max_id = max_id.max(id);
                }
            }
        }

        Ok(max_id + 1)
    }
    
    pub fn insert(&mut self, columns: Vec<Vec<Value>>) -> std::io::Result<()> {
        if columns.len() != self.writers.len() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "wrong column count"));
        }
        for (writer, vals) in self.writers.iter_mut().zip(columns) {
            writer.push_column(vals)?;
        }
        self.flush()
    }


    pub fn flush(&mut self) -> std::io::Result<()> {
        for w in &mut self.writers {
            w.flush()?;
        }
        Ok(())
    }
}