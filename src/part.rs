use std::path::{PathBuf};
use crate::schema::{ColumnDef};
use std::fs;

pub struct Part {
    pub dir: PathBuf, //e.g. data/events/part_00001
}

impl Part {
    pub fn new(dir: PathBuf) -> Self {
        Part { dir }
    }

    pub fn create_dir(&self) -> std::io::Result<()>{
        fs::create_dir_all(&self.dir)
    }

    pub fn column_bin_path(&self, col: &ColumnDef) -> PathBuf {
        self.dir.join(format!("{}.bin", col.name))
    }

    pub fn column_mrk_path(&self, col: &ColumnDef) -> PathBuf {
        self.dir.join(format!("{}.mrk", col.name))
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{ColumnDef, DataType, TableDef};
    use std::path::Path;

    #[test]
    fn part_paths() {
        let part = Part::new(PathBuf::from("data/events/part_00001"));

        let col = ColumnDef { name: "timestamp".to_string(), data_type: DataType::I64 };
        assert_eq!(part.column_bin_path(&col), PathBuf::from("data/events/part_00001/timestamp.bin"));
        assert_eq!(part.column_mrk_path(&col), PathBuf::from("data/events/part_00001/timestamp.mrk"));
    }

    #[test]
    fn part_dir_naming() {
        assert_eq!(
            TableDef::part_dir(Path::new("data/events"), 1),
            PathBuf::from("data/events/part_00001")
        );
        assert_eq!(
            TableDef::part_dir(Path::new("data/events"), 42),
            PathBuf::from("data/events/part_00042")
        );
    }
}
