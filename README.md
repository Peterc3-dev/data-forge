# data-forge

High-performance data processing CLI for CSV, JSON, JSONL, and TSV files.

## Features

- Column statistics: count, mean, median, min, max, nulls, inferred type
- Format conversion between CSV, JSON, JSONL, and TSV
- Row filtering with expressions (`column > value`, `column == value`)
- Sort by any column, ascending or descending
- Random sampling, head, and tail
- Schema inference and display
- Fast line counting via `bytecount`
- Memory-mapped I/O for large files
- Parallel processing with rayon
- Output as table, JSON, or CSV; quiet mode for scripting

## Install

```
cargo build --release
```

For native CPU optimization:

```
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

Binary lands at `target/release/data-forge`.

## Usage

```
data-forge stats data.csv                         # column statistics
data-forge convert input.csv output.json           # format conversion
data-forge filter data.tsv --where "age > 30"      # row filtering
data-forge sort data.csv --by revenue --desc        # sort
data-forge sample data.csv -n 500                  # random sample
data-forge head data.csv -n 20                     # first N rows
data-forge tail data.csv -n 5                      # last N rows
data-forge schema data.csv                         # infer schema
data-forge count data.csv                          # fast line count
data-forge stats data.csv --json                   # output as JSON
```

---

Built with Rust + rayon + memmap2.
