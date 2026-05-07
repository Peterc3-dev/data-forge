use anyhow::{Context, Result};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::fs::File;

use crate::color::Theme;
use crate::output::{self, OutputMode};
use crate::types;

pub fn run(path: &str, n: usize, mode: &OutputMode, theme: &Theme) -> Result<()> {
    let delimiter = types::detect_delimiter(path);

    let file = File::open(path).with_context(|| format!("Cannot open: {path}"))?;
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(true)
        .from_reader(file);

    let headers: Vec<String> = rdr.headers()?.iter().map(|s| s.to_string()).collect();

    let mut records: Vec<Vec<String>> = rdr
        .records()
        .filter_map(|r| r.ok())
        .map(|rec| rec.iter().map(|s| s.to_string()).collect())
        .collect();

    let total = records.len();
    let sample_size = n.min(total);

    // Fisher-Yates partial shuffle
    let mut rng = thread_rng();
    records.partial_shuffle(&mut rng, sample_size);
    records.truncate(sample_size);

    output::print_rows(&headers, &records, mode, theme);

    if *mode == OutputMode::Table {
        eprintln!(
            "{}",
            theme.dim(&format!("{sample_size} of {total} rows sampled"))
        );
    }

    Ok(())
}
