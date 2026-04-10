mod commands;
mod format;

use clap::Parser;
use commands::Cli;

fn main() {
    let cli = Cli::parse();
    commands::run(cli);
}
