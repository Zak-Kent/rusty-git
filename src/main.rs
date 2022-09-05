use clap::Parser;

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
    Tag
}

fn arg_to_gitcmd(arg: &str) -> Option<GitCmd> {
    match arg {
        "add"         => Some(GitCmd::Add),
        "cat-file"    => Some(GitCmd::CatFile),
        "checkout"    => Some(GitCmd::Checkout),
        "commit"      => Some(GitCmd::Commit),
        "hash-object" => Some(GitCmd::HashObject),
        "init"        => Some(GitCmd::Init),
        "log"         => Some(GitCmd::Log),
        "ls-tree"     => Some(GitCmd::LsTree),
        "merge"       => Some(GitCmd::Merge),
        "rebase"      => Some(GitCmd::Rebase),
        "rev-parse"   => Some(GitCmd::RevParse),
        "rm"          => Some(GitCmd::Rm),
        "show-ref"    => Some(GitCmd::ShowRef),
        "tag"         => Some(GitCmd::Tag),
        _             => None
    }
}


#[derive(Parser, Debug)]
struct Args {
    #[clap(value_parser)]
    cmd: String
}

fn main() {
    let args = Args::parse();
    println!("{:?}", args);

    let gcmd = arg_to_gitcmd(&args.cmd);
    println!("{:?}", gcmd);

    assert!(gcmd.is_some(), "{} is an invalid command!", args.cmd);

}
