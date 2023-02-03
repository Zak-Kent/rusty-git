# Rusty-git

This is a toy Git clone written in Rust primarily as a learning exercise
designed to increase my knowledge of internal Git data structures and to gain
some familiarity with the Rust language.

### Installing the tool
Written using Cargo and Rust 1.63.0
```
git clone https://github.com/Zak-Kent/rusty-git.git
cd src
cargo build
./target/debug/rusty-git --help
```

Assuming Cargo is present in your PATH the commands above should build the
project and all its dependencies placing the `rusty-git` executable in
`target/debug/` directory. From there the help command will show which commands
are available.

### .rusty-git-allowed file
Some operations which could be destructive to a real git repository are guarded
by the existance of a `.rusty-git-allowed` file. For example altering the
`.git/index` is guarded by requiring this file to exist in the top level of the
git repository. If the `rusty-git init` command is used to create a repository
this file will be created automatically otherwise a user may see an error when
attempting some commands related to this file not being found.

### Currently implemented commands
```
Commands:
  init         Create an empty git repo, errors if git repo already exists
  hash-object  Returns the sha1 hash of the file at the given path
  cat-file     Print the contents of the .git/objects file at the given sha
  log          Print commits starting at the given sha, defaults to HEAD
  ls-tree      Print contents of a tree object
  checkout     Checkout a given sha in a given directory, the directory must be empty and created beforehand
  show-ref     Display refs available in local repo along with associated commit IDs
  tag          Create or list tag objects
  ls-files     List the names of the files being tracked in the git index
  status       Show the working tree status
  add          Add file contents to the index
  commit       Record changes staged in the index to the repository
  help         Print this message or the help of the given subcommand(s)
```
