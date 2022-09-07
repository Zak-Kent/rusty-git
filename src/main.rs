use std::env;
use std::process;

// following tutorial here: https://wyag.thb.lt/

#[derive(Debug)]
enum GitCmd {
    Add,
    CatFile,
    Checkout,
    Commit,
    HashObject,
    Init,
    Log,
    LsTree,
    Merge,
    Rebase,
    RevParse,
    Rm,
    ShowRef,
    Tag,
}

impl GitCmd {
    fn new(cmd: &str) -> Result<GitCmd, &'static str> {
        match cmd {
            "add" => Ok(GitCmd::Add),
            "cat-file" => Ok(GitCmd::CatFile),
            "checkout" => Ok(GitCmd::Checkout),
            "commit" => Ok(GitCmd::Commit),
            "hash-object" => Ok(GitCmd::HashObject),
            "init" => Ok(GitCmd::Init),
            "log" => Ok(GitCmd::Log),
            "ls-tree" => Ok(GitCmd::LsTree),
            "merge" => Ok(GitCmd::Merge),
            "rebase" => Ok(GitCmd::Rebase),
            "rev-parse" => Ok(GitCmd::RevParse),
            "rm" => Ok(GitCmd::Rm),
            "show-ref" => Ok(GitCmd::ShowRef),
            "tag" => Ok(GitCmd::Tag),
            _ => Err("Command isn't supported"),
        }
    }
}

#[derive(Debug)]
struct Config {
    cmd: GitCmd,
    args: Vec<String>,
}

impl Config {
    fn new(cmdl: Vec<String>) -> Result<Config, &'static str> {
        if cmdl.len() == 1 {
            Err("No command entered")
        } else {
            let gcmd = GitCmd::new(&cmdl[1])?;
            Ok(Config {cmd: gcmd, args: cmdl[2..].to_vec()})
        }

    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);

    let cmd_config = Config::new(args).unwrap_or_else(|err: &str| {
        println!("Error: {}", err);
        process::exit(1);
    });
    println!("{:?}", cmd_config);

}
