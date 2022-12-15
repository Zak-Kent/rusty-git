use chrono::{DateTime, TimeZone, Utc};
use std::collections::HashSet;
use std::fs::{create_dir, create_dir_all, metadata, read, read_dir, read_to_string, File};
use std::io::{Error, Write};
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use std::str::from_utf8;
use tempfile::{tempdir, TempDir};

use crate::error as err;
use crate::object_parsers::{self as objp, NameSha};
use crate::objects as obj;

// ----------- test utils ---------------
pub fn test_tempdir() -> Result<TempDir, Error> {
    let tmp_dir = tempdir()?;
    Ok(tmp_dir)
}

#[allow(dead_code)]
pub fn test_gitdir() -> Result<TempDir, err::Error> {
    let dir = test_tempdir()?;
    create_git_repo(dir.path())?;
    return Ok(dir);
}

#[allow(dead_code)]
pub fn test_cmd(cmd: &str, arg: Option<&str>) -> Vec<String> {
    let mut cmd_result = Vec::from(["rusty-git".to_owned(), cmd.to_owned()]);

    if let Some(arg) = arg {
        cmd_result.push(arg.to_owned());
    }

    return cmd_result;
}

#[allow(dead_code)]
pub fn test_add_dummy_commit_and_update_ref_heads(
    sha: &str,
    repo: &obj::Repo,
) -> Result<(), err::Error> {
    //TODO: expand this to add an actual commit in .git/objects later
    let head_path = repo.gitdir.join("HEAD");
    let head = read(head_path)?;
    let head_ref = objp::parse_git_head(&head)?;
    let mut ref_file = File::create(repo.gitdir.join(head_ref))?;
    writeln!(ref_file, "{}", sha)?;
    Ok(())
}

// ----------- git utils ---------------
pub fn is_git_repo(path: &Path) -> bool {
    let gitdir = path.join(".git");
    let conf = path.join(".git/config");
    gitdir.exists() && conf.exists()
}

pub fn git_repo_or_err(path: &Path) -> Result<PathBuf, err::Error> {
    let gitrepo = is_git_repo(path);
    if gitrepo {
        return Ok(path.to_owned());
    } else {
        Err(err::Error::GitNotARepo)
    }
}

pub fn default_repo_config() -> &'static str {
    "[core]
       bare = false
       filemode = false
       repositoryformatversion = 0"
}

pub fn create_git_repo(path: &Path) -> Result<Option<String>, err::Error> {
    if is_git_repo(path) {
        return Err(err::Error::GitRepoAlreadyExists);
    }
    create_dir_all(path.join(".git/objects"))?;
    create_dir_all(path.join(".git/refs/heads"))?;
    create_dir_all(path.join(".git/refs/tags"))?;

    let mut head = File::create(path.join(".git/HEAD"))?;
    writeln!(head, "ref: refs/heads/master")?;

    let mut description = File::create(path.join(".git/description"))?;
    writeln!(
        description,
        "Unnamed repository; edit this file 'description' to name the repository."
    )?;

    let mut config = File::create(path.join(".git/config"))?;
    writeln!(config, "{}", default_repo_config())?;
    Ok(None)
}

pub fn git_obj_path_from_sha(sha: &str, repo: &obj::Repo) -> Result<PathBuf, err::Error> {
    let obj_path = repo
        .gitdir
        .join(format!("objects/{}/{}", &sha[..2], &sha[2..]));

    if obj_path.exists() {
        return Ok(obj_path.to_path_buf());
    } else {
        return Err(err::Error::GitObjPathDoesntExist(
            obj_path.display().to_string(),
        ));
    }
}

pub fn git_sha_from_head(repo: &obj::Repo) -> Result<String, err::Error> {
    let head_path = repo.gitdir.join("HEAD");
    let head = read(head_path)?;
    let head_ref = objp::parse_git_head(&head)?;
    let sha_path = repo.gitdir.join(head_ref);

    if sha_path.exists() {
        let sha = read_to_string(&sha_path)?.trim().to_owned();
        return Ok(sha);
    } else {
        return Err(err::Error::GitNoCommitsExistYet);
    };
}

pub fn git_read_commit(sha: &str, repo: &obj::Repo) -> Result<objp::KvsMsg, err::Error> {
    let commit = obj::read_object(sha, repo)?;
    let parsed_commit = objp::parse_kv_list_msg(&commit.contents, sha)?;
    return Ok(parsed_commit);
}

pub fn git_follow_commits_to_root(
    sha: &str,
    repo: &obj::Repo,
) -> Result<Vec<objp::KvsMsg>, err::Error> {
    let mut commit = git_read_commit(sha, &repo)?;
    let mut commit_log: Vec<objp::KvsMsg> = Vec::new();

    // add the first commit to log
    commit_log.push(commit.clone());

    while let Some(parent) = &commit.kvs.get("parent".as_bytes()) {
        let parent_sha = from_utf8(parent)?;
        commit = git_read_commit(parent_sha, &repo)?;

        // add parent commits to log
        commit_log.push(commit.clone());
    }

    // add root commit to log
    commit_log.push(commit.clone());

    return Ok(commit_log);
}

pub fn git_commit_to_string(commit: &objp::KvsMsg) -> Result<String, err::Error> {
    let mut output = String::new();
    output.push_str(&format!("commit: {}\n", commit.sha));
    output.push_str(&format!(
        "Author: {}\n",
        from_utf8(commit.kvs.get("author".as_bytes()).unwrap())?
    ));
    output.push_str(&format!("\n{}\n", from_utf8(&commit.msg)?));
    return Ok(output);
}

pub fn git_commit_log_to_string(commit_log: Vec<objp::KvsMsg>) -> Result<String, err::Error> {
    let mut output = String::new();
    for commit in commit_log {
        output.push_str(&git_commit_to_string(&commit)?);
    }
    return Ok(output);
}

pub fn git_tree_leaf_to_string(objp::TreeLeaf { mode, path, sha }: &objp::TreeLeaf) -> String {
    return format!("{mode} {sha} {path}\n");
}

pub fn git_tree_to_string(objp::Tree { contents }: objp::Tree) -> String {
    let mut output = String::new();
    for leaf in contents {
        output.push_str(&git_tree_leaf_to_string(&leaf));
    }
    return output;
}

pub fn git_get_tree_from_commit(
    sha: &str,
    contents: &[u8],
    repo: &obj::Repo,
) -> Result<objp::Tree, err::Error> {
    let objp::KvsMsg { kvs, .. } = objp::parse_kv_list_msg(contents, sha)?;

    let tree_sha = match kvs.get("tree".as_bytes()) {
        Some(s) => from_utf8(s)?,
        None => return Err(err::Error::GitNoTreeKeyInCommit),
    };

    let obj::GitObject { contents, .. } = obj::read_object(tree_sha, repo)?;
    let tree = objp::parse_git_tree(&contents)?;

    return Ok(tree);
}

pub fn git_checkout_tree(
    tree: objp::Tree,
    path: &Path,
    repo: &obj::Repo,
) -> Result<(), err::Error> {
    for leaf in tree.contents {
        let obj = obj::read_object(&leaf.sha, repo)?;

        match obj.obj {
            obj::GitObjTyp::Tree => {
                let sub_tree = objp::parse_git_tree(&obj.contents)?;
                let dir_path = path.join(&leaf.path);
                let dst = repo.worktree.join(&dir_path);
                create_dir(dst)?;
                git_checkout_tree(sub_tree, &dir_path, repo)?;
            }
            obj::GitObjTyp::Blob => {
                let dst = repo.worktree.join(path).join(&leaf.path);
                let mut dstfile = File::create(dst)?;
                dstfile.write_all(&obj.contents)?;
            }
            _ => return Err(err::Error::GitTreeInvalidObject),
        }
    }
    return Ok(());
}

pub fn git_resolve_ref(ref_path: &Path, repo: &obj::Repo) -> Result<String, err::Error> {
    let data = read_to_string(repo.gitdir.join(ref_path))?;
    if "ref: " == &data[..5] {
        git_resolve_ref(&PathBuf::from(data[5..].trim()), repo)
    } else {
        return Ok(data.trim().to_owned());
    }
}

pub fn git_gather_refs(path: Option<&Path>, repo: &obj::Repo) -> Result<Vec<String>, err::Error> {
    let refs_dir_path = if path == None {
        repo.gitdir.join("refs/")
    } else {
        path.unwrap().to_path_buf()
    };

    let mut all_refs: Vec<String> = Vec::new();
    let refs_dir = read_dir(refs_dir_path)?;

    for rf in refs_dir {
        let rfs_path = &rf?.path();
        let ref_md = metadata(rfs_path)?;

        if ref_md.is_dir() {
            let mut nested_refs = git_gather_refs(Some(rfs_path), repo)?;
            all_refs.append(&mut nested_refs);
        } else {
            // git_resolve_ref expects paths relative to .git/
            let clean_rf_path = rfs_path.strip_prefix(&repo.gitdir)?.to_owned();
            let resolved_ref = git_resolve_ref(&clean_rf_path, repo)?;
            if let Some(clean_path) = clean_rf_path.to_str() {
                all_refs.push(format!("{resolved_ref} {clean_path}\n"));
            } else {
                return Err(err::Error::PathToUtf8Conversion);
            };
        }
    }
    return Ok(all_refs);
}

pub fn git_list_all_tags(repo: &obj::Repo) -> Result<Vec<String>, err::Error> {
    let tags_path = repo.gitdir.join("refs/tags/");
    let tags = git_gather_refs(Some(&tags_path), &repo)?;
    return Ok(tags);
}

pub fn git_create_lightweight_tag(
    tag_name: &String,
    object: &String,
    repo: &obj::Repo,
) -> Result<(), err::Error> {
    let tag_sha: String;
    if object == "HEAD" {
        tag_sha = git_sha_from_head(repo)?;
    } else {
        tag_sha = object.to_owned();
    };
    let tag_path = repo.gitdir.join(format!("refs/tags/{}", tag_name));
    let mut tag = File::create(&tag_path)?;
    writeln!(tag, "{}", tag_sha)?;
    return Ok(());
}

pub fn git_read_index(repo: &obj::Repo) -> Result<Vec<u8>, err::Error> {
    let index_path = repo.gitdir.join("index");
    return Ok(read(index_path)?);
}

fn git_index_file_sha_pairs<T: objp::NameSha>(
    input: &Vec<T>,
    name_prefix: Option<String>,
) -> HashSet<(String, String)> {
    return input
        .iter()
        .map(|elm| elm.get_name_and_sha(name_prefix.clone()))
        .collect();
}

fn git_tree_file_sha_pairs(
    tree: objp::Tree,
    name_prefix: Option<String>,
    repo: &obj::Repo,
) -> Result<HashSet<(String, String)>, err::Error> {
    let mut file_sha_pairs: HashSet<(String, String)> = HashSet::new();
    // extra complexity needed to deal with nested git Tree objects
    for elm in tree.contents.iter() {
        if PathBuf::from(&elm.path).is_dir() {
            let obj::GitObject { obj, contents, .. } = obj::read_object(&elm.sha, &repo)?;
            if obj != obj::GitObjTyp::Tree {
                return Err(err::Error::GitLsTreeWrongObjType(format!("{:?}", obj)));
            } else {
                let nested_name_prefix: Option<String>;
                if let Some(ref nnp) = name_prefix {
                    nested_name_prefix = Some(format!("{}/{}", nnp, elm.path));
                } else {
                    nested_name_prefix = Some(elm.path.clone());
                }
                let tree = objp::parse_git_tree(&contents)?;
                let inner_tree_file_sha_pairs =
                    git_tree_file_sha_pairs(tree, nested_name_prefix, repo)?;
                file_sha_pairs.extend(inner_tree_file_sha_pairs);
            }
        } else {
            file_sha_pairs.insert(elm.get_name_and_sha(name_prefix.clone()));
        }
    }
    return Ok(file_sha_pairs);
}

fn git_staged_but_not_commited(repo: &obj::Repo, index: &objp::Index) -> Result<String, err::Error> {
    // get a set of (name, sha) pairs for each file in the last commit object
    let head_sha = git_sha_from_head(repo)?;
    let obj::GitObject { contents, sha, .. } = obj::read_object(&head_sha, &repo)?;
    let commit_tree = git_get_tree_from_commit(&sha, &contents, &repo)?;
    let commit_tree_files_n_shas = git_tree_file_sha_pairs(commit_tree, None, repo)?;

    // get set of (name, sha) pairs for each file in the index
    let index_files_n_shas: HashSet<(String, String)> =
        git_index_file_sha_pairs(&index.entries, None);

    return Ok(format!(
        "{}",
        index_files_n_shas
            .difference(&commit_tree_files_n_shas)
            .into_iter()
            .map(|(name, _)| format!("modified: {name}\n"))
            .collect::<String>()
    ));
}

fn git_ignored_files(repo: &obj::Repo) -> Result<HashSet<PathBuf>, err::Error> {
    let gitignore_path = repo.worktree.join(".gitignore");
    let gitignore = read(gitignore_path)?;

    let mut output: HashSet<PathBuf> = HashSet::new();
    for path in from_utf8(&gitignore)?.split('\n') {
        if path == "" {
            continue;
        } else {
            if path.starts_with("/") {
                output.insert(PathBuf::from(path[1..].to_owned()));
            } else {
                output.insert(PathBuf::from(path.to_owned()));
            }
        }
    }
    return Ok(output);
}

fn git_gather_mtime_from_worktree(
    path: Option<&Path>,
    repo: &obj::Repo,
) -> Result<HashSet<(String, DateTime<Utc>)>, err::Error> {
    let work_path = if path == None {
        repo.worktree.clone()
    } else {
        path.unwrap().to_path_buf()
    };

    let mut file_mtime_pairs: HashSet<(String, DateTime<Utc>)> = HashSet::new();
    let worktree_dir = read_dir(work_path)?;
    let ignored_files = git_ignored_files(repo)?;

    for node in worktree_dir {
        let node_val = node?;
        let node_path = &node_val.path();
        let node_name = &node_val.file_name();

        if node_name == ".git" || ignored_files.contains(node_path.strip_prefix(&repo.worktree)?) {
            continue;
        }

        let node_md = metadata(&node_val.path())?;
        if node_md.is_dir() {
            let inner_vals = git_gather_mtime_from_worktree(Some(node_path), repo)?;
            file_mtime_pairs.extend(inner_vals);
        } else {
            let node_mtime = node_md.modified()?;
            let node_dt: DateTime<Utc> = node_mtime.clone().into();
            let clean_node_path = node_path.strip_prefix(&repo.worktree)?;
            if let Some(node_path) = clean_node_path.to_str() {
                let file_output = (node_path.to_owned(), node_dt);
                file_mtime_pairs.insert(file_output);
            } else {
                return Err(err::Error::PathToUtf8Conversion);
            };
        }
    }
    return Ok(file_mtime_pairs);
}

fn git_local_changes_not_staged_for_commit(repo: &obj::Repo, index: &objp::Index) -> Result<String, err::Error> {
    let names_mtimes = index
        .entries
        .iter()
        .map(|objp::IndexEntry { name, m_time, .. }| (name.to_owned(), m_time.to_owned()));

    let idx_name_mtime_pairs: HashSet<(String, DateTime<Utc>)> = HashSet::from_iter(names_mtimes);
    let worktree_name_mtime_pairs = git_gather_mtime_from_worktree(None, repo)?;

    return Ok(format!(
        "{}",
        idx_name_mtime_pairs
            .difference(&worktree_name_mtime_pairs)
            .into_iter()
            .map(|(name, _)| format!("modified: {name}\n"))
            .collect::<String>()
    ));
}

pub fn git_status(repo: &obj::Repo) -> Result<String, err::Error> {
    let idx = git_read_index(repo)?;
    let index = objp::parse_git_index(&idx)?;

    let staged = git_staged_but_not_commited(repo, &index)?;
    let not_staged = git_local_changes_not_staged_for_commit(repo, &index)?;
    // TODO: need to add check for untracked files and update status
    let status = format!(
        "Changes to be committed:\n\n{}\nChanges not staged for commit:\n\n{}",
        staged, not_staged
    );
    return Ok(status);
}

// ----------- fs utils ---------------
pub fn build_path(mut path: PathBuf, ext: &str) -> Result<PathBuf, err::Error> {
    path.push(ext);
    if path.exists() {
        return Ok(path);
    } else {
        Err(err::Error::PathDoesntExist(path.display().to_string()))
    }
}

pub fn content_length(path: &Path) -> Result<u64, err::Error> {
    Ok(metadata(path)?.len())
}

pub fn dir_is_empty(path: &Path) -> Result<bool, err::Error> {
    return Ok(path.try_exists()? && path.read_dir()?.next().is_none());
}

fn dir_path_to_string(path: &Path) -> Result<String, err::Error> {
    if let Some(dir_name) = path.to_str() {
        return Ok(dir_name.to_owned());
    } else {
        println!("couldn't convert dir to str: {:?}", path);
        return Err(err::Error::DirNameToUtf8Conversion);
    }
}

pub fn dir_ok_for_checkout(path: &Path) -> Result<bool, err::Error> {
    match path.try_exists()? {
        true => true,
        false => return Err(err::Error::TargetDirDoesntExist(dir_path_to_string(path)?)),
    };

    if path.read_dir()?.next().is_none() {
        return Ok(true);
    } else {
        return Err(err::Error::TargetDirNotEmpty(dir_path_to_string(path)?));
    }
}

#[cfg(test)]
mod utils_tests {
    use ini;
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn return_true_when_git_repo() {
        // need the two var decs below to get around the borrow checker
        // not seeing that the ref .path() creates should be bound
        // through the unwrap when written: Result<TempDir>.unwrap().path()
        let gitdir = test_gitdir().unwrap();
        let gitdir_path = gitdir.path();
        assert!(is_git_repo(gitdir_path))
    }

    #[test]
    fn return_false_when_not_git_repo() {
        let tempdir = test_tempdir().unwrap();
        let tempdir_path = tempdir.path();
        assert!(!is_git_repo(tempdir_path))
    }

    #[test]
    fn create_gitdir_succeeds_in_empty_dir() {
        let tempdir = test_tempdir().unwrap();
        let tempdir_path = tempdir.path();

        let create_git_repo_result = create_git_repo(&tempdir_path);
        if create_git_repo_result.is_err() {
            panic!("repo setup failed in test!")
        }

        let gitdir_path = tempdir_path.join(".git");
        assert!(gitdir_path.try_exists().expect(".git dir should exist"));

        let assert_path = |ext: &str| {
            assert!(gitdir_path
                .join(ext)
                .try_exists()
                .unwrap_or_else(|err| panic!(".git/{} should exist. Error: {}", ext, err)))
        };
        assert_path("objects");
        assert_path("refs/tags");
        assert_path("refs/heads");
        assert_path("HEAD");
        assert_path("config");
        assert_path("description");

        let mut core: HashMap<String, Option<String>> = HashMap::new();
        core.insert("filemode".to_owned(), Some("false".to_owned()));
        core.insert("repositoryformatversion".to_owned(), Some("0".to_owned()));
        core.insert("bare".to_owned(), Some("false".to_owned()));

        let mut expected_config: HashMap<String, HashMap<String, Option<String>>> = HashMap::new();
        expected_config.insert("core".to_owned(), core);

        let config = ini::ini!(gitdir_path.join("config").to_str().unwrap());
        assert_eq!(expected_config, config);
    }

    #[test]
    fn create_gitdir_errors_in_an_existing_git_dir() {
        let gitdir = test_gitdir().unwrap();
        let gitdir_path = gitdir.path();
        assert!(is_git_repo(gitdir_path));

        let create_git_repo_result = create_git_repo(gitdir_path);
        assert!(Err(err::Error::GitRepoAlreadyExists) == create_git_repo_result);
    }

    #[test]
    fn dir_is_empty_works_as_expected() {
        let tempdir = test_tempdir().unwrap();
        let gitdir = test_gitdir().unwrap();
        assert_eq!(Ok(true), dir_is_empty(tempdir.path()));
        assert_eq!(Ok(false), dir_is_empty(gitdir.path()));
    }

    #[test]
    fn resolve_ref_follows_indirect_refs_until_direct_ref() {
        let gitdir = test_gitdir().unwrap();

        let foo_path = gitdir.path().join(".git/refs/heads/foo");
        let mut foo_ref = File::create(&foo_path).unwrap();
        writeln!(foo_ref, "ref: refs/heads/bar").unwrap();

        let direct_ref = "123shaABC";
        let mut bar_ref = File::create(gitdir.path().join(".git/refs/heads/bar")).unwrap();
        writeln!(bar_ref, "{}", &direct_ref).unwrap();

        let repo = obj::Repo::new(gitdir.path().to_path_buf()).unwrap();
        let resolved_ref = git_resolve_ref(&foo_path, &repo).unwrap();

        assert_eq!(direct_ref, resolved_ref);
    }

    #[test]
    fn can_create_and_read_lightweight_tags() {
        let gitdir = test_gitdir().unwrap();
        let repo = obj::Repo::new(gitdir.path().to_path_buf()).unwrap();

        let tag_sha = "0e6cfc8b4209c9ecca33dbd30c41d1d4289736e1".to_owned();
        git_create_lightweight_tag(&"foo".to_owned(), &tag_sha, &repo).unwrap();

        let tag = git_list_all_tags(&repo).unwrap();
        let expected = format!("{tag_sha} refs/tags/foo\n");
        assert_eq!(&expected, tag.first().unwrap());
    }
}
