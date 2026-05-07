use anyhow::Result;
use std::fs::File;
use std::io::{BufReader, Read};

use crate::color::Theme;
use crate::output::OutputMode;

/// Fast line count using SIMD-accelerated newline scanning (via bytecount crate).
/// Uses buffered file reading instead of mmap to avoid SIGBUS on concurrent truncation.
pub fn run(path: &str, mode: &OutputMode, theme: &Theme) -> Result<()> {
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);

    let mut newline_count: usize = 0;
    let mut total_bytes: usize = 0;
    let mut last_byte: Option<u8> = None;
    let mut buf = [0u8; 64 * 1024];

    loop {
        let bytes_read = reader.read(&mut buf)?;
        if bytes_read == 0 {
            break;
        }
        newline_count += bytecount::count(&buf[..bytes_read], b'\n');
        total_bytes += bytes_read;
        last_byte = Some(buf[bytes_read - 1]);
    }

    // If file doesn't end with newline, the last line still counts
    let line_count = if total_bytes == 0 {
        0
    } else if last_byte == Some(b'\n') {
        newline_count
    } else {
        newline_count + 1
    };

    match mode {
        OutputMode::Quiet => println!("{line_count}"),
        OutputMode::Json => {
            let obj = serde_json::json!({
                "file": path,
                "lines": line_count,
            });
            println!("{}", obj);
        }
        OutputMode::Csv => {
            println!("file,lines");
            println!("{},{}", path, line_count);
        }
        OutputMode::Table => {
            println!(
                "{} {}",
                theme.bright(&line_count.to_string()),
                theme.dim("lines")
            );
        }
    }

    Ok(())
}
