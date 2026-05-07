use anyhow::{Context, Result};
use std::fs::File;

use crate::color::Theme;
use crate::output::{self, OutputMode};
use crate::types::{self, ColumnType};

pub fn run(path: &str, mode: &OutputMode, theme: &Theme) -> Result<()> {
    let delimiter = types::detect_delimiter(path);

    let file = File::open(path).with_context(|| format!("Cannot open: {path}"))?;
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(true)
        .from_reader(file);

    let headers: Vec<String> = rdr.headers()?.iter().map(|s| s.to_string()).collect();
    let num_cols = headers.len();

    // Sample up to 10000 rows for type inference
    let max_sample = 10_000;
    let mut col_types: Vec<ColumnType> = vec![ColumnType::Empty; num_cols];
    let mut row_count = 0usize;
    let mut null_counts = vec![0usize; num_cols];

    for result in rdr.records().take(max_sample) {
        if let Ok(record) = result {
            row_count += 1;
            for (i, field) in record.iter().enumerate() {
                if i >= num_cols {
                    break;
                }
                if types::is_null_value(field) {
                    null_counts[i] += 1;
                } else {
                    let cell_type = types::infer_cell_type(field);
                    col_types[i] = types::merge_types(&col_types[i], &cell_type);
                }
            }
        }
    }

    // Determine nullable
    let schema: Vec<(String, String, bool)> = headers
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let nullable = null_counts[i] > 0;
            (name.clone(), col_types[i].to_string(), nullable)
        })
        .collect();

    match mode {
        OutputMode::Json => {
            let mut result = Vec::new();
            for (name, dtype, nullable) in &schema {
                let mut map = serde_json::Map::new();
                map.insert("column".into(), serde_json::Value::String(name.clone()));
                map.insert("type".into(), serde_json::Value::String(dtype.clone()));
                map.insert("nullable".into(), serde_json::Value::Bool(*nullable));
                result.push(serde_json::Value::Object(map));
            }
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputMode::Csv => {
            println!("column,type,nullable");
            for (name, dtype, nullable) in &schema {
                println!("{name},{dtype},{nullable}");
            }
        }
        OutputMode::Quiet => {
            for (name, dtype, nullable) in &schema {
                let null_marker = if *nullable { "?" } else { "" };
                println!("{name}\t{dtype}{null_marker}");
            }
        }
        OutputMode::Table => {
            let table_headers = vec![
                "Column".to_string(),
                "Type".to_string(),
                "Nullable".to_string(),
            ];
            let rows: Vec<Vec<String>> = schema
                .iter()
                .map(|(name, dtype, nullable)| {
                    vec![
                        name.clone(),
                        dtype.clone(),
                        if *nullable {
                            "yes".to_string()
                        } else {
                            "no".to_string()
                        },
                    ]
                })
                .collect();

            output::print_table(&table_headers, &rows, theme);

            eprintln!(
                "{}",
                theme.dim(&format!(
                    "Inferred from {} rows, {} columns",
                    row_count, num_cols
                ))
            );
        }
    }

    Ok(())
}
