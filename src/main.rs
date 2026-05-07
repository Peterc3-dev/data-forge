mod cli;
mod color;
mod convert;
mod count;
mod filter;
mod headtail;
mod output;
mod sample;
mod schema;
mod sort;
mod stats;
mod types;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Command};
use color::Theme;
use output::OutputMode;

fn main() -> Result<()> {
    let cli = Cli::parse();

    let theme = Theme::new(!cli.no_color);
    let mode = if cli.quiet {
        OutputMode::Quiet
    } else if cli.json {
        OutputMode::Json
    } else if cli.csv {
        OutputMode::Csv
    } else {
        OutputMode::Table
    };

    match &cli.command {
        Command::Stats { file } => stats::run(file, &mode, &theme),
        Command::Convert { input, output } => convert::run(input, output, &theme),
        Command::Filter { file, condition } => filter::run(file, condition, &mode, &theme),
        Command::Sort { file, by, desc } => sort::run(file, by, *desc, &mode, &theme),
        Command::Sample { file, n } => sample::run(file, *n, &mode, &theme),
        Command::Head { file, n } => headtail::head(file, *n, &mode, &theme),
        Command::Tail { file, n } => headtail::tail(file, *n, &mode, &theme),
        Command::Schema { file } => schema::run(file, &mode, &theme),
        Command::Count { file } => count::run(file, &mode, &theme),
    }
}
