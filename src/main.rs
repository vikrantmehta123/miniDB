mod aggregator;
mod config;
mod encoding;
mod executor;
mod parser;
mod storage;

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use storage::schema::TableDef;

fn dump_part(table_dir: &std::path::Path, part_id: u32, schema: &storage::schema::TableDef) {
    use storage::column_chunk::ColumnChunk;
    use storage::table_reader::TableReader;

    let mut reader = TableReader::open(table_dir, part_id).unwrap();
    println!("--- part_{:05} ---", part_id);
    for g in 0..reader.granule_count() {
        let chunks = reader.read_granule(g).unwrap();
        let n = chunks.first().map(|c| c.len()).unwrap_or(0);
        for row in 0..n {
            let values: Vec<String> = chunks
                .iter()
                .map(|c| match c {
                    ColumnChunk::I64(v) => v[row].to_string(),
                    ColumnChunk::U32(v) => v[row].to_string(),
                    ColumnChunk::Bool(v) => v[row].to_string(),
                    ColumnChunk::Str(v) => format!("'{}'", v[row]),
                    _ => "?".into(),
                })
                .collect();
            println!("  ({})", values.join(", "));
        }
    }
}

fn main() -> io::Result<()> {
    let table_dir = PathBuf::from("data").join("tinyolap_smoke");
    std::fs::create_dir_all(&table_dir)?;

    let schema = TableDef::open(&table_dir).unwrap_or_else(|_| {
        eprintln!("No schema.json found in {:?}. Create one first.", table_dir);
        std::process::exit(1);
    });

    println!("tinyOLAP ready. Table: '{}'", schema.name);
    println!("Type SQL and press Enter. Ctrl-D to quit.\n");

    let stdin = io::stdin();
    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break; // EOF
        }
        let sql = line.trim();
        if sql.is_empty() {
            continue;
        }

        match crate::parser::parse(sql) {
            Err(e) => eprintln!("parse error: {e:?}"),
            Ok(crate::parser::Statement::Insert(stmt)) => {
                match executor::execute_insert(stmt, &schema, table_dir.clone()) {
                    Ok(meta) => {
                        println!("OK ({} rows inserted, part_{})", meta.rows, meta.part_id);
                        dump_part(&table_dir, meta.part_id, &schema);
                    }

                    Err(e) => eprintln!("error: {e}"),
                }
            }
        }
    }

    Ok(())
}
