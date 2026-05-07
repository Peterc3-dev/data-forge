use comfy_table::{Table, ContentArrangement, presets};
use crate::color::Theme;

/// Output mode for commands.
#[derive(Debug, Clone, PartialEq)]
pub enum OutputMode {
    Table,
    Json,
    Csv,
    Quiet,
}

/// Print rows as a pretty table with phosphor-green styling.
pub fn print_table(headers: &[String], rows: &[Vec<String>], theme: &Theme) {
    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);

    let styled_headers: Vec<String> = headers.iter().map(|h| theme.header(h)).collect();
    table.set_header(styled_headers);

    for row in rows {
        let styled: Vec<String> = row.iter().map(|c| theme.green(c)).collect();
        table.add_row(styled);
    }

    println!("{table}");
}

/// Print rows as JSON array.
pub fn print_json(headers: &[String], rows: &[Vec<String>]) {
    let mut records: Vec<serde_json::Map<String, serde_json::Value>> = Vec::with_capacity(rows.len());
    for row in rows {
        let mut map = serde_json::Map::new();
        for (i, val) in row.iter().enumerate() {
            let key = headers.get(i).cloned().unwrap_or_else(|| format!("col_{i}"));
            map.insert(key, serde_json::Value::String(val.clone()));
        }
        records.push(map);
    }
    let json = serde_json::to_string_pretty(&records).unwrap_or_else(|_| "[]".to_string());
    println!("{json}");
}

/// Print rows as CSV.
pub fn print_csv(headers: &[String], rows: &[Vec<String>]) {
    let mut wtr = csv::Writer::from_writer(std::io::stdout());
    let _ = wtr.write_record(headers);
    for row in rows {
        let _ = wtr.write_record(row);
    }
    let _ = wtr.flush();
}

/// Print rows in the given output mode.
pub fn print_rows(headers: &[String], rows: &[Vec<String>], mode: &OutputMode, theme: &Theme) {
    match mode {
        OutputMode::Table => print_table(headers, rows, theme),
        OutputMode::Json => print_json(headers, rows),
        OutputMode::Csv => print_csv(headers, rows),
        OutputMode::Quiet => {
            // Quiet: just row count
            println!("{}", rows.len());
        }
    }
}

/// Print a single key-value list (for stats, schema, etc.).
#[allow(dead_code)]
pub fn print_kv_table(pairs: &[(String, String)], theme: &Theme) {
    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![theme.header("Property"), theme.header("Value")]);

    for (k, v) in pairs {
        table.add_row(vec![theme.dim(k), theme.value(v)]);
    }

    println!("{table}");
}

/// Print a multi-column stats table (one column per data column).
pub fn print_stats_table(
    stat_names: &[&str],
    columns: &[String],
    values: &[Vec<String>],
    theme: &Theme,
) {
    let mut table = Table::new();
    table.load_preset(presets::UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);

    let mut header = vec![theme.header("Statistic")];
    for col in columns {
        header.push(theme.header(col));
    }
    table.set_header(header);

    for (i, stat_name) in stat_names.iter().enumerate() {
        let mut row = vec![theme.dim(stat_name)];
        for col_vals in values {
            let val = col_vals.get(i).cloned().unwrap_or_default();
            row.push(theme.value(&val));
        }
        table.add_row(row);
    }

    println!("{table}");
}
