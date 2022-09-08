use std::env;
use std::process;
mod config;

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);

    let cmd_config = config::Config::new(args).unwrap_or_else(|err: &str| {
        println!("Error: {}", err);
        process::exit(1);
    });
    println!("{:?}", cmd_config);

}
