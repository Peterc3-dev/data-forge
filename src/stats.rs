use anyhow::Result;
use rayon::prelude::*;
use std::fs::File;

use crate::color::Theme;
use crate::output::{self, OutputMode};
use crate::types::{self, ColumnType};

/// Per-column statistics accumulator.
struct ColumnStats {
    name: String,
    count: usize,
    null_count: usize,
    numeric_values: Vec<f64>,
    inferred_type: ColumnType,
    min_str: Option<String>,
    max_str: Option<String>,
}

impl ColumnStats {
    fn new(name: String) -> Self {
        Self {
            name,
            count: 0,
            null_count: 0,
            numeric_values: Vec::new(),
            inferred_type: ColumnType::Empty,
            min_str: None,
            max_str: None,
        }
    }

    fn add(&mut self, value: &str) {
        self.count += 1;

        if types::is_null_value(value) {
            self.null_count += 1;
            return;
        }

        let cell_type = types::infer_cell_type(value);
        self.inferred_type = types::merge_types(&self.inferred_type, &cell_type);

        if let Ok(n) = value.trim().parse::<f64>() {
            self.numeric_values.push(n);
        }

        let trimmed = value.trim().to_string();
        match &self.min_str {
            None => self.min_str = Some(trimmed.clone()),
            Some(cur) if trimmed < *cur => self.min_str = Some(trimmed.clone()),
            _ => {}
        }
        match &self.max_str {
            None => self.max_str = Some(trimmed),
            Some(cur) if trimmed > *cur => self.max_str = Some(trimmed),
            _ => {}
        }
    }

    fn mean(&self) -> Option<f64> {
        if self.numeric_values.is_empty() {
            return None;
        }
        let sum: f64 = self.numeric_values.iter().sum();
        Some(sum / self.numeric_values.len() as f64)
    }

    fn median(&self) -> Option<f64> {
        if self.numeric_values.is_empty() {
            return None;
        }
        let mut sorted = self.numeric_values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let len = sorted.len();
        if len % 2 == 0 {
            Some((sorted[len / 2 - 1] + sorted[len / 2]) / 2.0)
        } else {
            Some(sorted[len / 2])
        }
    }

    fn min_val(&self) -> Option<f64> {
        self.numeric_values
            .iter()
            .copied()
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    }

    fn max_val(&self) -> Option<f64> {
        self.numeric_values
            .iter()
            .copied()
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    }
}

pub fn run(path: &str, mode: &OutputMode, theme: &Theme) -> Result<()> {
    let delimiter = types::detect_delimiter(path);
    let file = File::open(path)?;
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(true)
        .from_reader(file);

    let headers: Vec<String> = rdr.headers()?.iter().map(|s| s.to_string()).collect();
    let num_cols = headers.len();

    // Collect all records into memory for parallel processing
    let records: Vec<csv::StringRecord> = rdr.records().filter_map(|r| r.ok()).collect();

    // Process each column in parallel
    let col_stats: Vec<ColumnStats> = (0..num_cols)
        .into_par_iter()
        .map(|col_idx| {
            let mut stats = ColumnStats::new(headers[col_idx].clone());
            for record in &records {
                let val = record.get(col_idx).unwrap_or("");
                stats.add(val);
            }
            stats
        })
        .collect();

    // Output
    match mode {
        OutputMode::Json => {
            let mut result = Vec::new();
            for s in &col_stats {
                let mut map = serde_json::Map::new();
                map.insert("column".into(), serde_json::Value::String(s.name.clone()));
                map.insert(
                    "type".into(),
                    serde_json::Value::String(s.inferred_type.to_string()),
                );
                map.insert(
                    "count".into(),
                    serde_json::json!(s.count),
                );
                map.insert(
                    "null_count".into(),
                    serde_json::json!(s.null_count),
                );
                if let Some(m) = s.mean() {
                    map.insert("mean".into(), serde_json::json!(m));
                }
                if let Some(m) = s.median() {
                    map.insert("median".into(), serde_json::json!(m));
                }
                if let Some(m) = s.min_val() {
                    map.insert("min".into(), serde_json::json!(m));
                }
                if let Some(m) = s.max_val() {
                    map.insert("max".into(), serde_json::json!(m));
                }
                result.push(serde_json::Value::Object(map));
            }
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputMode::Csv => {
            println!("column,type,count,null_count,mean,median,min,max");
            for s in &col_stats {
                println!(
                    "{},{},{},{},{},{},{},{}",
                    s.name,
                    s.inferred_type,
                    s.count,
                    s.null_count,
                    s.mean().map_or("".into(), |v| format!("{v:.4}")),
                    s.median().map_or("".into(), |v| format!("{v:.4}")),
                    s.min_val().map_or(
                        s.min_str.clone().unwrap_or_default(),
                        |v| format!("{v}")
                    ),
                    s.max_val().map_or(
                        s.max_str.clone().unwrap_or_default(),
                        |v| format!("{v}")
                    ),
                );
            }
        }
        OutputMode::Quiet => {
            for s in &col_stats {
                println!(
                    "{}\t{}\t{}\t{}",
                    s.name,
                    s.count,
                    s.null_count,
                    s.mean().map_or("N/A".into(), |v| format!("{v:.4}"))
                );
            }
        }
        OutputMode::Table => {
            let stat_names = &["type", "count", "null_count", "mean", "median", "min", "max"];
            let col_names: Vec<String> = col_stats.iter().map(|s| s.name.clone()).collect();
            let values: Vec<Vec<String>> = col_stats
                .iter()
                .map(|s| {
                    vec![
                        s.inferred_type.to_string(),
                        s.count.to_string(),
                        s.null_count.to_string(),
                        s.mean().map_or("-".into(), |v| format!("{v:.4}")),
                        s.median().map_or("-".into(), |v| format!("{v:.4}")),
                        s.min_val().map_or(
                            s.min_str.clone().unwrap_or("-".into()),
                            |v| format!("{v}"),
                        ),
                        s.max_val().map_or(
                            s.max_str.clone().unwrap_or("-".into()),
                            |v| format!("{v}"),
                        ),
                    ]
                })
                .collect();

            output::print_stats_table(stat_names, &col_names, &values, theme);
        }
    }

    Ok(())
}
