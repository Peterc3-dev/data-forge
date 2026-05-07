use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "data-forge",
    about = "High-performance data processing CLI",
    version,
    author
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Disable color output
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Output as JSON
    #[arg(long, global = true)]
    pub json: bool,

    /// Output as CSV
    #[arg(long, global = true)]
    pub csv: bool,

    /// Quiet mode — minimal output
    #[arg(long, short, global = true)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Column statistics (count, mean, median, min, max, null count, type)
    Stats {
        /// Input file (CSV, TSV)
        file: String,
    },

    /// Convert between CSV, JSON, JSONL, TSV formats
    Convert {
        /// Input file
        input: String,
        /// Output file
        output: String,
    },

    /// Filter rows matching a condition
    Filter {
        /// Input file
        file: String,
        /// Filter expression: "column > value", "column == value", etc.
        #[arg(long = "where")]
        condition: String,
    },

    /// Sort rows by a column
    Sort {
        /// Input file
        file: String,
        /// Column to sort by
        #[arg(long)]
        by: String,
        /// Sort descending
        #[arg(long)]
        desc: bool,
    },

    /// Random sample of N rows
    Sample {
        /// Input file
        file: String,
        /// Number of rows to sample
        #[arg(long, short, default_value = "1000")]
        n: usize,
    },

    /// First N rows
    Head {
        /// Input file
        file: String,
        /// Number of rows
        #[arg(long, short, default_value = "10")]
        n: usize,
    },

    /// Last N rows
    Tail {
        /// Input file
        file: String,
        /// Number of rows
        #[arg(long, short, default_value = "10")]
        n: usize,
    },

    /// Infer and display schema
    Schema {
        /// Input file
        file: String,
    },

    /// Fast line count
    Count {
        /// Input file
        file: String,
    },
}
