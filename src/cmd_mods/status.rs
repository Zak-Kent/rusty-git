use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::fs::{metadata, read, read_dir};
use std::path::{Path, PathBuf};
use std::str::from_utf8;

use crate::error as err;
use crate::index as idx;
use crate::objects::{self as obj, tree, NameSha};
use crate::utils;

fn index_file_sha_pairs<T: obj::NameSha>(
    input: &Vec<T>,
    name_prefix: Option<String>,
) -> HashSet<(String, String)> {
    return input
        .iter()
        .map(|elm| elm.get_name_and_sha(name_prefix.clone()))
        .collect();
}

fn tree_file_sha_pairs(
    tree: tree::Tree,
    name_prefix: Option<String>,
    repo: &obj::Repo,
) -> Result<HashSet<(String, String)>, err::Error> {
    let mut file_sha_pairs: HashSet<(String, String)> = HashSet::new();
    // extra complexity needed to deal with nested git Tree objects
    for elm in tree.contents.iter() {
        if PathBuf::from(&elm.path).is_dir() {
            let obj = obj::read_object(&utils::get_sha_from_binary(&elm.sha), repo)?;
            match obj {
                obj::GitObj::Tree(inner_tree) => {
                    let nested_name_prefix: Option<String>;
                    if let Some(ref nnp) = name_prefix {
                        nested_name_prefix = Some(format!("{}/{}", nnp, elm.path));
                    } else {
                        nested_name_prefix = Some(elm.path.clone());
                    }
                    let inner_tree_file_sha_pairs =
                        tree_file_sha_pairs(inner_tree, nested_name_prefix, repo)?;
                    file_sha_pairs.extend(inner_tree_file_sha_pairs);
                }
                _ => return Err(err::Error::GitLsTreeWrongObjType(format!("{:?}", obj))),
            }
        } else {
            file_sha_pairs.insert(elm.get_name_and_sha(name_prefix.clone()));
        }
    }
    return Ok(file_sha_pairs);
}

pub fn staged_but_not_commited(repo: &obj::Repo, index: &idx::Index) -> Result<String, err::Error> {
    let commit_tree_files_n_shas: HashSet<(String, String)>;
    let head_sha = utils::git_sha_from_head(repo);

    if let Ok(hsha) = head_sha {
        // get a set of (name, sha) pairs for each file in the last commit object
        if let obj::GitObj::Commit(commit) = obj::read_object(&hsha, repo)? {
            let commit_tree = utils::git_get_tree_from_commit(commit, &repo)?;
            commit_tree_files_n_shas = tree_file_sha_pairs(commit_tree, None, repo)?;
        } else {
            return Err(err::Error::GitUnexpectedInternalType(format!(
                "{:?}",
                "Expected a commit object"
            )));
        }
    } else {
        // This error happens when no commits exist yet and you try to look
        // them up from HEAD. This can happen when running status before
        // a commit is added.
        assert!(head_sha.err() == Some(err::Error::GitNoCommitsExistYet));
        commit_tree_files_n_shas = HashSet::new();
    };

    // get set of (name, sha) pairs for each file in the index
    let index_files_n_shas: HashSet<(String, String)> = index_file_sha_pairs(&index.entries, None);

    return Ok(format!(
        "{}",
        index_files_n_shas
            .difference(&commit_tree_files_n_shas)
            .into_iter()
            .map(|(name, _)| format!("modified: {name}\n"))
            .collect::<String>()
    ));
}

fn ignored_files(repo: &obj::Repo) -> Result<HashSet<PathBuf>, err::Error> {
    let gitignore_path = repo.worktree.join(".gitignore");
    // if no gitignore return empty hashset
    if !gitignore_path.exists() {
        return Ok(HashSet::new());
    }

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

fn gather_mtime_from_worktree(
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
    let ignored_files = ignored_files(repo)?;

    for node in worktree_dir {
        let node_val = node?;
        let node_path = &node_val.path();
        let node_name = &node_val.file_name();

        if node_name == ".git" || ignored_files.contains(node_path.strip_prefix(&repo.worktree)?) {
            continue;
        }

        let node_md = metadata(&node_val.path())?;
        if node_md.is_dir() {
            let inner_vals = gather_mtime_from_worktree(Some(node_path), repo)?;
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

struct LocalChanges {
    not_staged: String,
    not_tracked: String,
}

fn local_changes_not_staged_for_commit_or_untracked(
    repo: &obj::Repo,
    index: &idx::Index,
) -> Result<LocalChanges, err::Error> {
    let names_mtimes = index
        .entries
        .iter()
        .map(|idx::IndexEntry { name, m_time, .. }| (name.to_owned(), m_time.to_owned()));

    let idx_name_mtime_pairs: HashSet<(String, DateTime<Utc>)> = HashSet::from_iter(names_mtimes);
    let worktree_name_mtime_pairs = gather_mtime_from_worktree(None, repo)?;

    let not_staged = format!(
        "{}",
        idx_name_mtime_pairs
            .difference(&worktree_name_mtime_pairs)
            .into_iter()
            .map(|(name, _)| format!("modified: {name}\n"))
            .collect::<String>()
    );

    let not_tracked = format!(
        "{}",
        worktree_name_mtime_pairs
            .difference(&idx_name_mtime_pairs)
            .into_iter()
            .map(|(name, _)| format!("{name}\n"))
            .collect::<String>()
    );

    return Ok(LocalChanges {
        not_staged,
        not_tracked,
    });
}

pub fn status(repo: &obj::Repo) -> Result<String, err::Error> {
    if !utils::git_index_exists(repo) {
        return Ok("Nothing in the stagging area!
             The .git/index file doesn't yet exist try:
             'rusty-git add <file-name>' to trigger index creation"
            .to_owned());
    }

    let idx = utils::git_read_index(repo)?;
    let index = idx::parse_git_index(&idx)?;

    let staged = staged_but_not_commited(repo, &index)?;
    let LocalChanges {
        not_staged,
        not_tracked,
    } = local_changes_not_staged_for_commit_or_untracked(repo, &index)?;
    let status = format!(
        "Changes to be committed:\n\n{}\n\
         Changes not staged for commit:\n\n{}\n\
         Untracked files:\n\n{}",
        staged, not_staged, not_tracked
    );
    return Ok(status);
}
