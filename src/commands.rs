use std::path::{Path, PathBuf};

use crate::cli;
use crate::cmd_mods::{add, checkout, init, log, lstree, refs, status, tag, commit as cmt};
use crate::error as err;
use crate::index as idx;
use crate::objects::{self as obj, blob};
use crate::utils;

fn run_init(cmd: &cli::Cli) -> Result<Option<String>, err::Error> {
    let repo_path = PathBuf::from(&cmd.repo_path);
    return Ok(init::create_git_repo(&repo_path)?);
}

fn hash_object(
    path: String,
    repo: obj::Repo,
    write_obj: bool,
) -> Result<Option<String>, err::Error> {
    let bpath: PathBuf = PathBuf::from(path);
    let blob = blob::blob_from_path(bpath)?;

    // by passing None to write_obj it will only return the hash, no write
    let repo_arg;
    if write_obj {
        repo_arg = Some(&repo);
    } else {
        repo_arg = None;
    }
    return Ok(Some(obj::write_object(blob, repo_arg)?.to_string()));
}

// This version of cat-file differs from git's due to the fact git expects
// the object type in the args for the cmd, e.g, 'git cat-file <obj type> <sha>'
// where this version only needs the sha and then reads the obj type from
// the compressed file stored at the sha's location
fn cat_file(sha: String, repo: obj::Repo) -> Result<Option<String>, err::Error> {
    let file_contents = obj::read_object_as_string(&sha, &repo)?;
    return Ok(Some(file_contents));
}

fn log(sha: String, repo: obj::Repo) -> Result<Option<String>, err::Error> {
    let target_commit = match sha.as_str() {
        "HEAD" => utils::git_sha_from_head(&repo)?,
        _ => sha,
    };
    let commit_log = log::follow_commits_to_root(&target_commit, &repo)?;
    let output = log::commit_log_to_string(commit_log)?;
    return Ok(Some(output));
}

fn lstree(sha: String, repo: obj::Repo) -> Result<Option<String>, err::Error> {
    let obj = obj::read_object(&sha, &repo)?;

    if let obj::GitObj::Tree(tree) = obj {
        let output = lstree::git_tree_to_string(tree);
        return Ok(Some(output));
    } else {
        return Err(err::Error::GitLsTreeWrongObjType(format!("{:?}", obj)));
    }
}

fn checkout(sha: &str, dir: &Path, repo: obj::Repo) -> Result<Option<String>, err::Error> {
    checkout::dir_ok_for_checkout(dir)?;
    let obj = obj::read_object(&sha, &repo)?;
    match obj {
        obj::GitObj::Tree(tree) => {
            checkout::checkout_tree(tree, dir, &repo)?;
        }
        obj::GitObj::Commit(commit) => {
            let tree = utils::git_get_tree_from_commit(commit, &repo)?;
            checkout::checkout_tree(tree, dir, &repo)?;
        }
        _ => return Err(err::Error::GitCheckoutWrongObjType(format!("{:?}", obj))),
    }
    return Ok(None);
}

fn show_ref(repo: obj::Repo) -> Result<Option<String>, err::Error> {
    let refs = refs::gather_refs(None, &repo)?.concat();
    return Ok(Some(refs));
}

fn tag(
    name: &Option<String>,
    object: &String,
    add_object: &bool,
    repo: obj::Repo,
) -> Result<Option<String>, err::Error> {
    if let Some(n) = name {
        if *add_object {
            return Err(err::Error::GitCreateTagObjectNotImplemented);
        } else {
            tag::create_lightweight_tag(n, object, &repo)?;
            return Ok(None);
        }
    } else {
        return Ok(Some(tag::list_all_tags(&repo)?.concat()));
    }
}

pub fn ls_files(repo: obj::Repo) -> Result<Option<String>, err::Error> {
    let index_contents = utils::git_read_index(&repo)?;
    let index = idx::parse_git_index(&index_contents)?;
    let file_names: Vec<String> = index
        .entries
        .into_iter()
        .map(|e| format!("{}\n", e.name))
        .collect();
    return Ok(Some(file_names.concat()));
}

pub fn status(repo: obj::Repo) -> Result<Option<String>, err::Error> {
    let status = status::status(&repo)?;
    return Ok(Some(status));
}

pub fn add(file_name: String, repo: obj::Repo) -> Result<Option<String>, err::Error> {
    // don't mess with index unless user opts in
    utils::git_check_for_rusty_git_allowed(&repo)?;

    // 'git add' hashes the file and adds it to .git/objects
    hash_object(file_name.clone(), repo.clone(), true)?;

    let index_exists = utils::git_index_exists(&repo);
    if index_exists {
        let _file_exists = utils::build_path(repo.worktree.clone(), &file_name)?;
        add::update_index(&repo, &file_name)?;
    } else {
        // index doesn't exist yet and must be created
        let entry = add::file_to_index_entry(&file_name, &repo)?;
        let index = idx::Index::new(entry)?;
        add::write_index(index, &repo)?;
    }
    return Ok(None);
}

fn commit(msg: String, repo: obj::Repo) -> Result<Option<String>, err::Error> {
    // don't allow commits unless user opts in
    utils::git_check_for_rusty_git_allowed(&repo)?;

    cmt::commit(msg, repo)?;
    return Ok(None);
}

pub fn run_cmd(cmd: &cli::Cli, write_obj: bool) -> Result<Option<String>, err::Error> {
    let command = &cmd.command;
    let repo: Option<obj::Repo>;

    // unwrap calls to repo below safe because of this check
    if cmd.command != cli::GitCmd::Init {
        repo = Some(obj::Repo::new(PathBuf::from(cmd.repo_path.to_owned()))?);
    } else {
        repo = None;
    }

    match command {
        cli::GitCmd::Init => run_init(&cmd),
        cli::GitCmd::HashObject { path } => hash_object(path.to_owned(), repo.unwrap(), write_obj),
        cli::GitCmd::CatFile { sha } => cat_file(sha.to_owned(), repo.unwrap()),
        cli::GitCmd::Log { sha } => log(sha.to_owned(), repo.unwrap()),
        cli::GitCmd::LsTree { sha } => lstree(sha.to_owned(), repo.unwrap()),
        cli::GitCmd::Checkout { sha, dir } => checkout(sha, Path::new(dir), repo.unwrap()),
        cli::GitCmd::ShowRef => show_ref(repo.unwrap()),
        cli::GitCmd::Tag {
            name,
            object,
            add_object,
        } => tag(name, object, add_object, repo.unwrap()),
        cli::GitCmd::LsFiles => ls_files(repo.unwrap()),
        cli::GitCmd::Status => status(repo.unwrap()),
        cli::GitCmd::Add { file_name } => add(file_name.to_owned(), repo.unwrap()),
        cli::GitCmd::Commit { msg } => commit(msg.to_string(), repo.unwrap()),
    }
}

#[cfg(test)]
mod object_tests {
    use std::collections::HashSet;
    use std::fs::read;
    use std::fs::File;
    use std::io::Write;

    use super::*;
    use crate::test_utils;
    use crate::utils;

    #[test]
    fn hash_object_returns_hash_and_cat_file_reads_test() -> Result<(), err::Error> {
        let worktree = test_utils::test_gitdir().unwrap();

        let fp = worktree.path().join("tempfoo");
        let mut tmpfile = File::create(&fp)?;
        writeln!(tmpfile, "foobar")?;

        let cmd = cli::Cli {
            command: cli::GitCmd::HashObject {
                path: fp.to_str().unwrap().to_owned(),
            },
            repo_path: worktree.path().to_str().unwrap().to_owned(),
        };

        let hash = run_cmd(&cmd, true)?;

        assert_eq!(
            hash,
            Some("323fae03f4606ea9991df8befbb2fca795e648fa".to_owned())
        );

        let cat_cmd = cli::Cli {
            command: cli::GitCmd::CatFile { sha: hash.unwrap() },
            repo_path: worktree.path().to_str().unwrap().to_owned(),
        };

        let file_contents = run_cmd(&cat_cmd, false)?;
        assert_eq!(file_contents, Some("foobar\n".to_owned()));
        Ok(())
    }

    #[test]
    fn can_read_sha_from_head() -> Result<(), err::Error> {
        // TODO: expand this test to cover the log command when added
        let worktree = test_utils::test_gitdir().unwrap();
        let repo = obj::Repo::new(worktree.path().to_path_buf())?;

        test_utils::test_add_dummy_commit_and_update_ref_heads(&"fake-head-sha", &repo)?;

        let head_sha = utils::git_sha_from_head(&repo)?;
        assert_eq!("fake-head-sha", head_sha);
        Ok(())
    }

    #[test]
    fn can_add_a_new_file_to_existing_index() {
        let gitdir = test_utils::test_gitdir_with_index().unwrap();
        let repo = obj::Repo::new(gitdir.path().to_path_buf()).unwrap();

        let starting_index = read(gitdir.path().join(".git/index")).unwrap();
        let parsed_starting_index = idx::parse_git_index(&starting_index).unwrap();
        let mut starting_file_names: HashSet<String> = HashSet::new();
        for e in parsed_starting_index.entries {
            starting_file_names.insert(e.name);
        }

        let new_file_name = "foo-aleady-exists-in-fake-index.txt";
        let new_file = File::create(repo.worktree.join(new_file_name));
        writeln!(new_file.unwrap(), "{}", "hahaha").unwrap();

        let updated_index = add::add_entry_to_index(&repo, new_file_name).unwrap();
        let mut updated_file_names: HashSet<String> = HashSet::new();
        for e in updated_index.entries {
            updated_file_names.insert(e.name);
        }

        assert_eq!(
            1,
            updated_file_names.difference(&starting_file_names).count()
        );
        assert_eq!(
            new_file_name,
            updated_file_names
                .difference(&starting_file_names)
                .last()
                .unwrap()
        )
    }

    #[test]
    fn can_create_index_when_first_file_added() {
        let gitdir = test_utils::test_gitdir().unwrap();
        let repo = obj::Repo::new(gitdir.path().to_path_buf()).unwrap();

        let new_file_name = "foo.txt";
        let new_file_full_path = repo.worktree.join(new_file_name);
        let new_file = File::create(new_file_full_path.clone());
        writeln!(new_file.unwrap(), "{}", "hahaha").unwrap();

        let add_cmd = cli::Cli {
            command: cli::GitCmd::Add {
                file_name: new_file_full_path.clone().to_str().unwrap().to_owned(),
            },
            repo_path: repo.worktree.to_str().unwrap().to_owned(),
        };

        // .git/index file doesn't exist before add cmd is run
        let index_exists = &gitdir.path().join(".git/index").exists();
        assert!(!index_exists);

        let git_objects_empty = &gitdir
            .path()
            .join(".git/objects")
            .read_dir()
            .unwrap()
            .next()
            .is_none();
        assert!(git_objects_empty);

        // running add command creates a blob object and .git/index file
        run_cmd(&add_cmd, false).unwrap();

        let index = read(gitdir.path().join(".git/index")).unwrap();
        let parsed_index = idx::parse_git_index(&index).unwrap();
        assert_eq!(1, parsed_index.entries.len());
        assert_eq!(
            new_file_full_path.to_str().unwrap(),
            parsed_index.entries.first().unwrap().name.as_str()
        );

        let git_objects = gitdir.path().join(".git/objects").read_dir().unwrap();
        assert_eq!(1, git_objects.count());
    }
}
