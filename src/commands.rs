use crate::config as cfg;
use crate::error as err;
use crate::utils;

fn run_init(config: &cfg::Config) -> Result<(), err::Error> {
    return Ok(utils::create_git_repo(&config.path)?);
}

pub fn run_cmd(config: &cfg::Config) -> Result<(), err::Error> {
    match config.cmd {
        cfg::GitCmd::Init => return run_init(&config),
        _ => return Err(err::Error::UnimplementedCommand)
    }
}
