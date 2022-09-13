use std::env;
use std::process;

mod config;
mod objects;
mod utils;

use crate::config as cfg;
use crate::objects as obj;

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("passed in args: {:?}", args);

    let cmd_config = cfg::Config::new(args, None).unwrap_or_else(|err: &str| {
        println!("Error: {}", err);
        process::exit(1);
    });
    println!("Config struct: {:?}", cmd_config);

    let repo = obj::Repo::new(cmd_config).unwrap_or_else(|err: String| {
        println!("Error creating git repo: {}", err);
        process::exit(1);
    });
    println!("{:?}", repo);
}
