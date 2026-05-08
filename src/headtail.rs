use anyhow::{Context, Result};
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};

use crate::color::Theme;
use crate::output::{self, OutputMode};
use crate::types;

pub fn head(path: &str, n: usize, mode: &OutputMode, theme: &Theme) -> Result<()> {
    let delimiter = types::detect_delimiter(path);

    let file = File::open(path).with_context(|| format!("Cannot open: {path}"))?;
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(true)
        .from_reader(file);

    let headers: Vec<String> = rdr.headers()?.iter().map(|s| s.to_string()).collect();

    // Stream and take only the first N records — never loads more than N rows.
    let rows: Vec<Vec<String>> = rdr
        .records()
        .filter_map(|r| r.ok())
        .take(n)
        .map(|rec| rec.iter().map(|s| s.to_string()).collect())
        .collect();

    output::print_rows(&headers, &rows, mode, theme);

    if *mode == OutputMode::Table {
        eprintln!("{}", theme.dim(&format!("First {} rows", rows.len())));
    }

    Ok(())
}

pub fn tail(path: &str, n: usize, mode: &OutputMode, theme: &Theme) -> Result<()> {
    let delimiter = types::detect_delimiter(path);

    let file = File::open(path).with_context(|| format!("Cannot open: {path}"))?;
    let meta = file.metadata()?;
    let file_size = meta.len();

    // For files under 100MB, stream through with a ring buffer of size N.
    // This avoids loading the entire file into memory — only N rows are held
    // at any time, regardless of file size.
    if file_size < 100 * 1024 * 1024 {
        let mut rdr = csv::ReaderBuilder::new()
            .delimiter(delimiter)
            .flexible(true)
            .from_reader(file);

        let headers: Vec<String> = rdr.headers()?.iter().map(|s| s.to_string()).collect();

        // Use a ring buffer (VecDeque) to keep only the last N rows in memory.
        let mut ring: VecDeque<Vec<String>> = VecDeque::with_capacity(n + 1);

        for result in rdr.records() {
            let record = match result {
                Ok(r) => r,
                Err(_) => continue,
            };
            let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
            ring.push_back(row);
            if ring.len() > n {
                ring.pop_front();
            }
        }

        let rows: Vec<Vec<String>> = ring.into_iter().collect();

        output::print_rows(&headers, &rows, mode, theme);

        if *mode == OutputMode::Table {
            eprintln!("{}", theme.dim(&format!("Last {} rows", rows.len())));
        }
    } else {
        // For large files: read header, then seek backwards to avoid
        // streaming through the entire file.
        let mut file = File::open(path)?;
        let mut header_reader = BufReader::new(&file);
        let mut header_line = String::new();
        header_reader.read_line(&mut header_line)?;
        let header_offset = header_line.len() as u64;
        drop(header_reader);

        // Parse header
        let mut hdr_rdr = csv::ReaderBuilder::new()
            .delimiter(delimiter)
            .has_headers(false)
            .from_reader(header_line.as_bytes());
        let headers: Vec<String> = if let Some(Ok(rec)) = hdr_rdr.records().next() {
            rec.iter().map(|s| s.to_string()).collect()
        } else {
            vec![]
        };

        // Seek backwards to find enough lines (overestimate to handle quoted fields
        // with embedded newlines that consume more raw bytes per record).
        let chunk_size: u64 = (n as u64 + 2) * 4096;
        let seek_pos = if file_size > chunk_size + header_offset {
            file_size - chunk_size
        } else {
            header_offset
        };

        file.seek(SeekFrom::Start(seek_pos))?;
        let mut raw_chunk = Vec::new();
        std::io::Read::read_to_end(&mut file, &mut raw_chunk)?;

        // If we seeked into the middle of the file we likely landed mid-record.
        // Drop bytes up to (and including) the first newline so the csv reader
        // starts on a record boundary.  When seek_pos == header_offset we are
        // already at a boundary, so skip nothing.
        let chunk_start = if seek_pos > header_offset {
            match raw_chunk.iter().position(|&b| b == b'\n') {
                Some(pos) => pos + 1,
                None => 0,
            }
        } else {
            0
        };
        let clean_chunk = &raw_chunk[chunk_start..];

        // Parse the chunk with the csv crate so quoted fields with embedded
        // newlines are handled correctly.
        let mut chunk_rdr = csv::ReaderBuilder::new()
            .delimiter(delimiter)
            .has_headers(false)
            .flexible(true)
            .from_reader(clean_chunk);

        // Use a ring buffer here too, in case the chunk has more than N records.
        let mut ring: VecDeque<Vec<String>> = VecDeque::with_capacity(n + 1);

        for result in chunk_rdr.records() {
            let record = match result {
                Ok(r) => r,
                Err(_) => continue,
            };
            let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
            ring.push_back(row);
            if ring.len() > n {
                ring.pop_front();
            }
        }

        let rows: Vec<Vec<String>> = ring.into_iter().collect();

        output::print_rows(&headers, &rows, mode, theme);

        if *mode == OutputMode::Table {
            eprintln!("{}", theme.dim(&format!("Last {} rows", rows.len())));
        }
    }

    Ok(())
}
