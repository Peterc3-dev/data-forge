use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fs::File;

use crate::color::Theme;
use crate::output::{self, OutputMode};
use crate::types;

pub fn run(
    path: &str,
    by_column: &str,
    descending: bool,
    mode: &OutputMode,
    theme: &Theme,
) -> Result<()> {
    let delimiter = types::detect_delimiter(path);

    let file = File::open(path).with_context(|| format!("Cannot open: {path}"))?;
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(true)
        .from_reader(file);

    let headers: Vec<String> = rdr.headers()?.iter().map(|s| s.to_string()).collect();

    let col_idx = headers
        .iter()
        .position(|h| h == by_column)
        .with_context(|| {
            format!(
                "Column '{}' not found. Available: {}",
                by_column,
                headers.join(", ")
            )
        })?;

    let mut records: Vec<Vec<String>> = rdr
        .records()
        .filter_map(|r| r.ok())
        .map(|rec| rec.iter().map(|s| s.to_string()).collect())
        .collect();

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} Sorting {msg}")
            .unwrap(),
    );
    pb.set_message(format!("{} rows by '{by_column}'", records.len()));

    // Parallel sort
    records.par_sort_by(|a, b| {
        let va = a.get(col_idx).map(|s| s.as_str()).unwrap_or("");
        let vb = b.get(col_idx).map(|s| s.as_str()).unwrap_or("");

        // Try numeric sort first
        if let (Ok(na), Ok(nb)) = (va.parse::<f64>(), vb.parse::<f64>()) {
            let cmp = na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal);
            if descending {
                return cmp.reverse();
            }
            return cmp;
        }

        // String sort
        let cmp = va.cmp(vb);
        if descending {
            cmp.reverse()
        } else {
            cmp
        }
    });

    pb.finish_and_clear();

    output::print_rows(&headers, &records, mode, theme);

    Ok(())
}
