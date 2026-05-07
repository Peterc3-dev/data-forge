use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

use crate::color::Theme;
use crate::types::FileFormat;

pub fn run(input_path: &str, output_path: &str, theme: &Theme) -> Result<()> {
    let in_fmt = FileFormat::from_path(input_path)
        .with_context(|| format!("Cannot detect input format: {input_path}"))?;
    let out_fmt = FileFormat::from_path(output_path)
        .with_context(|| format!("Cannot detect output format: {output_path}"))?;

    // Read all records
    let (headers, rows) = read_all(input_path, &in_fmt)?;

    let pb = ProgressBar::new(rows.len() as u64);
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} [{bar:40.green}] {pos}/{len} rows")
            .unwrap()
            .progress_chars("=> "),
    );

    // Write
    write_all(output_path, &out_fmt, &headers, &rows, &pb)?;

    pb.finish_and_clear();
    eprintln!(
        "{} {} rows from {} to {}",
        theme.bright("Converted"),
        theme.value(&rows.len().to_string()),
        theme.dim(input_path),
        theme.dim(output_path)
    );

    Ok(())
}

fn read_all(path: &str, fmt: &FileFormat) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    match fmt {
        FileFormat::Csv | FileFormat::Tsv => {
            let delim = if *fmt == FileFormat::Tsv { b'\t' } else { b',' };
            let file = File::open(path)?;
            let mut rdr = csv::ReaderBuilder::new()
                .delimiter(delim)
                .flexible(true)
                .from_reader(file);

            let headers: Vec<String> = rdr.headers()?.iter().map(|s| s.to_string()).collect();
            let rows: Vec<Vec<String>> = rdr
                .records()
                .filter_map(|r| r.ok())
                .map(|rec| rec.iter().map(|s| s.to_string()).collect())
                .collect();

            Ok((headers, rows))
        }
        FileFormat::Json => {
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            let data: serde_json::Value = serde_json::from_reader(reader)?;

            match data {
                serde_json::Value::Array(arr) => {
                    let mut headers: Vec<String> = Vec::new();
                    let mut rows: Vec<Vec<String>> = Vec::new();

                    // First pass: collect all keys
                    for item in &arr {
                        if let serde_json::Value::Object(map) = item {
                            for key in map.keys() {
                                if !headers.contains(key) {
                                    headers.push(key.clone());
                                }
                            }
                        }
                    }

                    // Second pass: extract values
                    for item in &arr {
                        if let serde_json::Value::Object(map) = item {
                            let row: Vec<String> = headers
                                .iter()
                                .map(|h| {
                                    map.get(h)
                                        .map(|v| value_to_string(v))
                                        .unwrap_or_default()
                                })
                                .collect();
                            rows.push(row);
                        }
                    }

                    Ok((headers, rows))
                }
                _ => anyhow::bail!("JSON file must contain an array of objects"),
            }
        }
        FileFormat::Jsonl => {
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            let mut headers: Vec<String> = Vec::new();
            let mut rows: Vec<Vec<String>> = Vec::new();

            // Two-pass approach for JSONL
            let lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();

            for line in &lines {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Ok(serde_json::Value::Object(map)) = serde_json::from_str(trimmed) {
                    for key in map.keys() {
                        if !headers.contains(key) {
                            headers.push(key.clone());
                        }
                    }
                }
            }

            for line in &lines {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Ok(serde_json::Value::Object(map)) = serde_json::from_str(trimmed) {
                    let row: Vec<String> = headers
                        .iter()
                        .map(|h| {
                            map.get(h)
                                .map(|v| value_to_string(v))
                                .unwrap_or_default()
                        })
                        .collect();
                    rows.push(row);
                }
            }

            Ok((headers, rows))
        }
    }
}

fn write_all(
    path: &str,
    fmt: &FileFormat,
    headers: &[String],
    rows: &[Vec<String>],
    pb: &ProgressBar,
) -> Result<()> {
    match fmt {
        FileFormat::Csv | FileFormat::Tsv => {
            let delim = if *fmt == FileFormat::Tsv { b'\t' } else { b',' };
            let file = File::create(path)?;
            let mut wtr = csv::WriterBuilder::new()
                .delimiter(delim)
                .from_writer(file);

            wtr.write_record(headers)?;
            for row in rows {
                wtr.write_record(row)?;
                pb.inc(1);
            }
            wtr.flush()?;
        }
        FileFormat::Json => {
            let file = File::create(path)?;
            let mut writer = BufWriter::new(file);

            let mut arr: Vec<serde_json::Map<String, serde_json::Value>> =
                Vec::with_capacity(rows.len());
            for row in rows {
                let mut map = serde_json::Map::new();
                for (i, val) in row.iter().enumerate() {
                    let key = headers.get(i).cloned().unwrap_or_else(|| format!("col_{i}"));
                    map.insert(key, serde_json::Value::String(val.clone()));
                }
                arr.push(map);
                pb.inc(1);
            }

            serde_json::to_writer_pretty(&mut writer, &arr)?;
            writer.write_all(b"\n")?;
            writer.flush()?;
        }
        FileFormat::Jsonl => {
            let file = File::create(path)?;
            let mut writer = BufWriter::new(file);

            for row in rows {
                let mut map = serde_json::Map::new();
                for (i, val) in row.iter().enumerate() {
                    let key = headers.get(i).cloned().unwrap_or_else(|| format!("col_{i}"));
                    map.insert(key, serde_json::Value::String(val.clone()));
                }
                serde_json::to_writer(&mut writer, &map)?;
                writer.write_all(b"\n")?;
                pb.inc(1);
            }
            writer.flush()?;
        }
    }

    Ok(())
}

fn value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}
