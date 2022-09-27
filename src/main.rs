use std::env;
use std::process;

mod config;
mod objects;
mod utils;
mod error;

use crate::config as cfg;
use crate::objects as obj;
use crate::error as err;

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("passed in args: {:?}", args);

    let cmd_config = cfg::Config::new(args, None).unwrap_or_else(|err: err::Error| {
        println!("Error: {}", err);
        process::exit(1);
    });
    println!("Config struct: {:?}", cmd_config);

    let repo = obj::Repo::new(cmd_config).unwrap_or_else(|err: err::Error| {
        println!("Error creating git repo: {}", err);
        process::exit(1);
    });
    println!("{:?}", repo);
}
