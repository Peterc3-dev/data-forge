use anyhow::{Context, Result};
use std::fs::File;

use crate::color::Theme;
use crate::output::{self, OutputMode};
use crate::types;

/// Supported filter operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    for (op_str, op) in operators {
        if let Some(idx) = expr.find(op_str) {
            let column = expr[..idx].trim().to_string();
            let value = expr[idx + op_str.len()..].trim().to_string();
            // Strip quotes from value if present
            let value = value.trim_matches('"').trim_matches('\'').to_string();
            return Ok(Condition { column, op, value });
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
        Op::Contains => cell_trimmed.to_lowercase().contains(&val.to_lowercase()),
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
        eprintln!("{}", theme.dim(&format!("{} rows matched", filtered.len())));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_operators_and_strips_quotes() {
        let c = parse_condition("age >= 30").unwrap();
        assert_eq!(c.column, "age");
        assert_eq!(c.op, Op::Gte);
        assert_eq!(c.value, "30");

        let c = parse_condition("name == \"Alice\"").unwrap();
        assert_eq!(c.column, "name");
        assert_eq!(c.op, Op::Eq);
        assert_eq!(c.value, "Alice");

        let c = parse_condition("city ~= 'york'").unwrap();
        assert_eq!(c.op, Op::Contains);
        assert_eq!(c.value, "york");
    }

    #[test]
    fn longest_operator_wins() {
        // ">=" must be matched before ">".
        assert_eq!(parse_condition("a >= 1").unwrap().op, Op::Gte);
        // "!=" must be matched before any single char.
        assert_eq!(parse_condition("a != 1").unwrap().op, Op::Neq);
    }

    #[test]
    fn rejects_expression_without_operator() {
        assert!(parse_condition("just a column").is_err());
    }

    #[test]
    fn numeric_comparison() {
        let cond = parse_condition("x > 10").unwrap();
        assert!(matches_condition("11", &cond));
        assert!(!matches_condition("10", &cond));
        assert!(!matches_condition("9", &cond));
        // Numeric value compared regardless of whitespace.
        assert!(matches_condition("  42 ", &cond));
    }

    #[test]
    fn string_comparison_and_contains() {
        let eq = parse_condition("name == Bob").unwrap();
        assert!(matches_condition("Bob", &eq));
        assert!(!matches_condition("bob", &eq));

        // Contains on non-numeric is case-insensitive.
        let contains = parse_condition("name ~= ob").unwrap();
        assert!(matches_condition("Bob", &contains));
        assert!(matches_condition("ROB", &contains));
        assert!(!matches_condition("Alice", &contains));
    }
}
