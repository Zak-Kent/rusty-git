use std::path::Path;

#[derive(Debug)]
pub enum GitCmd {
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
pub struct Config<'a> {
    pub cmd: GitCmd,
    pub path: Box<&'a Path>,
    pub args: Vec<String>,
}

impl Config<'_> {
    pub fn new(cmdl: Vec<String>) -> Result<Config<'static>, &'static str> {
        if cmdl.len() == 1 {
            Err("No command entered")
        } else {
            let gcmd = GitCmd::new(&cmdl[1])?;
            Ok(Config {
                cmd: gcmd,
                path: Box::new(Path::new(".")),
                args: cmdl[2..].to_vec(),
            })
        }
    }
}
