use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use crate::cmd_mods::status;
use crate::error as err;
use crate::index as idx;
use crate::objects::{self as obj, blob, commit, tree};
use crate::utils;

pub fn commit(msg: String, repo: obj::Repo) -> Result<Option<String>, err::Error> {
    let index_exists = utils::git_index_exists(&repo);
    if index_exists {
        let index_contents = utils::git_read_index(&repo)?;
        let index = idx::parse_git_index(&index_contents)?;

        // check if there are staged files that need to be committed
        let files_to_commit = status::staged_but_not_commited(&repo, &index)?;
        if files_to_commit == "" {
            println!("Nothing added to commit! Run 'rusty-git status' to see state of index.");
            return Ok(None);
        }

        let tree = tree::index_to_tree(&index);
        let tree_sha = obj::write_object(obj::GitObj::Tree(tree.clone()), Some(&repo))?;

        // make sure blobs exist for all files in tree
        for elm in tree.contents {
            let elm_path = PathBuf::from(elm.path);
            let blob = blob::blob_from_path(elm_path)?;
            obj::write_object(blob, Some(&repo))?;
        }

        let parent;
        if let Ok(head_sha) = utils::git_sha_from_head(&repo) {
            parent = Some(head_sha)
        } else {
            parent = None
        }

        let mut commit = commit::Commit {
            tree: tree_sha.to_string(),
            parent: parent.clone(),
            author: commit::create_dummy_user(),
            committer: commit::create_dummy_user(),
            msg: msg.clone(),
            sha: "".to_string(),
        };
        commit.calc_and_update_sha();
        obj::write_object(obj::GitObj::Commit(commit.clone()), Some(&repo))?;

        // write commit to ref path in HEAD
        let ref_path = utils::git_head_ref_path(&repo)?;
        // create will truncate the sha in the ref file if it previously existed
        let mut ref_file = File::create(ref_path)?;
        ref_file.write(commit.sha.as_bytes())?;
    } else {
        return Ok(Some(
            "Nothing in the stagging area!
             The .git/index file doesn't yet exist try:
             'rusty-git add <file-name>' to trigger index creation"
                .to_owned(),
        ));
    }

    Ok(None)
}
