use clap::Parser;
use std::process;

mod cli;
mod commands;
mod error;
mod index;
mod test_utils;
mod utils;
mod cmds;
mod objects;

use crate::commands as cmd;
use crate::error as err;

fn main() {
    let cli = cli::Cli::parse();
    let output = cmd::run_cmd(&cli, false).unwrap_or_else(|err: err::Error| {
        println!("Error: {}", err);
        process::exit(1);
    });

    if let Some(out) = output {
        println!("{}", out);
        process::exit(0);
    } else {
        process::exit(0);
    }
}
