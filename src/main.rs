use std::env;
use std::process;

mod commands;
mod config;
mod error;
mod objects;
mod utils;
mod object_parsers;

use crate::commands as cmd;
use crate::config as cfg;
use crate::error as err;

fn main() {
    let args: Vec<String> = env::args().collect();

    let cmd_config = cfg::Config::new(args, None).unwrap_or_else(|err: err::Error| {
        println!("Error: {}", err);
        process::exit(1);
    });

    let output = cmd::run_cmd(&cmd_config).unwrap_or_else(|err: err::Error| {
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
