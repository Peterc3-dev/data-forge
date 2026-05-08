use anyhow::{Context, Result};
use std::fs::File;

use crate::color::Theme;
use crate::output::{self, OutputMode};
use crate::types;

/// Supported filter operators.
#[derive(Debug)]
enum Op {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Contains,
}

/// Parsed filter condition.
struct Condition {
    column: String,
    op: Op,
    value: String,
}

fn parse_condition(expr: &str) -> Result<Condition> {
    // Try operators from longest to shortest to avoid partial matches
    let operators = [
        ("!=", Op::Neq),
        (">=", Op::Gte),
        ("<=", Op::Lte),
        ("==", Op::Eq),
        (">", Op::Gt),
        ("<", Op::Lt),
        ("~=", Op::Contains),
    ];

    for (op_str, op) in &operators {
        if let Some(idx) = expr.find(op_str) {
            let column = expr[..idx].trim().to_string();
            let value = expr[idx + op_str.len()..].trim().to_string();
            // Strip quotes from value if present
            let value = value.trim_matches('"').trim_matches('\'').to_string();
            return Ok(Condition {
                column,
                op: match op {
                    Op::Eq => Op::Eq,
                    Op::Neq => Op::Neq,
                    Op::Gt => Op::Gt,
                    Op::Gte => Op::Gte,
                    Op::Lt => Op::Lt,
                    Op::Lte => Op::Lte,
                    Op::Contains => Op::Contains,
                },
                value,
            });
        }
    }

    anyhow::bail!(
        "Invalid filter expression: '{expr}'. Use: column > value, column == value, column ~= value, etc."
    )
}

fn matches_condition(cell: &str, cond: &Condition) -> bool {
    let cell_trimmed = cell.trim();
    let val = &cond.value;

    // Try numeric comparison first
    if let (Ok(cell_num), Ok(val_num)) = (cell_trimmed.parse::<f64>(), val.parse::<f64>()) {
        return match cond.op {
            Op::Eq => (cell_num - val_num).abs() < f64::EPSILON,
            Op::Neq => (cell_num - val_num).abs() >= f64::EPSILON,
            Op::Gt => cell_num > val_num,
            Op::Gte => cell_num >= val_num,
            Op::Lt => cell_num < val_num,
            Op::Lte => cell_num <= val_num,
            Op::Contains => cell_trimmed.contains(val.as_str()),
        };
    }

    // Fall back to string comparison
    match cond.op {
        Op::Eq => cell_trimmed == val.as_str(),
        Op::Neq => cell_trimmed != val.as_str(),
        Op::Gt => cell_trimmed > val.as_str(),
        Op::Gte => cell_trimmed >= val.as_str(),
        Op::Lt => cell_trimmed < val.as_str(),
        Op::Lte => cell_trimmed <= val.as_str(),
        Op::Contains => cell_trimmed
            .to_lowercase()
            .contains(&val.to_lowercase()),
    }
}

pub fn run(path: &str, condition_str: &str, mode: &OutputMode, theme: &Theme) -> Result<()> {
    let cond = parse_condition(condition_str)?;
    let delimiter = types::detect_delimiter(path);

    let file = File::open(path).with_context(|| format!("Cannot open: {path}"))?;
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(true)
        .from_reader(file);

    let headers: Vec<String> = rdr.headers()?.iter().map(|s| s.to_string()).collect();

    let col_idx = headers
        .iter()
        .position(|h| h == &cond.column)
        .with_context(|| {
            format!(
                "Column '{}' not found. Available: {}",
                cond.column,
                headers.join(", ")
            )
        })?;

    // Stream records one at a time, filtering without loading all into memory.
    let mut filtered: Vec<Vec<String>> = Vec::new();

    for result in rdr.records() {
        let record = match result {
            Ok(r) => r,
            Err(_) => continue,
        };
        let matches = record
            .get(col_idx)
            .map(|cell| matches_condition(cell, &cond))
            .unwrap_or(false);
        if matches {
            let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
            filtered.push(row);
        }
    }

    output::print_rows(&headers, &filtered, mode, theme);

    if *mode == OutputMode::Table {
        eprintln!(
            "{}",
            theme.dim(&format!("{} rows matched", filtered.len()))
        );
    }

    Ok(())
}
