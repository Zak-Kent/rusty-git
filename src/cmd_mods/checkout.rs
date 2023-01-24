use std::fs::{create_dir, File};
use std::io::Write;
use std::path::Path;

use crate::error as err;
use crate::object_parsers as objp;
use crate::objects as obj;
use crate::utils;

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

pub fn checkout_tree(tree: objp::Tree, path: &Path, repo: &obj::Repo) -> Result<(), err::Error> {
    for leaf in tree.contents {
        let obj = obj::read_object(&utils::get_sha_from_binary(&leaf.sha), repo)?;

        match obj.obj {
            obj::GitObjTyp::Tree => {
                let sub_tree = objp::parse_git_tree(&obj.contents)?;
                let dir_path = path.join(&leaf.path);
                let dst = repo.worktree.join(&dir_path);
                create_dir(dst)?;
                checkout_tree(sub_tree, &dir_path, repo)?;
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
