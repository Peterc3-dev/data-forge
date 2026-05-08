use anyhow::{Context, Result};
use rand::Rng;
use std::fs::File;

use crate::color::Theme;
use crate::output::{self, OutputMode};
use crate::types;

/// Reservoir sampling (Algorithm R, Vitter 1985).
/// Produces a uniform random sample of k items from an iterator of unknown length
/// in a single pass with O(k) memory.
pub fn run(path: &str, n: usize, mode: &OutputMode, theme: &Theme) -> Result<()> {
    let delimiter = types::detect_delimiter(path);

    let file = File::open(path).with_context(|| format!("Cannot open: {path}"))?;
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(true)
        .from_reader(file);

    let headers: Vec<String> = rdr.headers()?.iter().map(|s| s.to_string()).collect();

    let mut rng = rand::thread_rng();
    let mut reservoir: Vec<Vec<String>> = Vec::with_capacity(n);
    let mut total: usize = 0;

    for result in rdr.records() {
        let record = match result {
            Ok(r) => r,
            Err(_) => continue,
        };
        let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
        total += 1;

        if total <= n {
            // Fill the reservoir with the first k items
            reservoir.push(row);
        } else {
            // For item i (1-indexed), replace a random reservoir element
            // with probability k/i
            let j = rng.gen_range(0..total);
            if j < n {
                reservoir[j] = row;
            }
        }
    }

    let sample_size = reservoir.len();

    output::print_rows(&headers, &reservoir, mode, theme);

    if *mode == OutputMode::Table {
        eprintln!(
            "{}",
            theme.dim(&format!("{sample_size} of {total} rows sampled"))
        );
    }

    Ok(())
}
