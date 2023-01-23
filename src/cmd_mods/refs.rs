use std::fs::{metadata, read_dir, read_to_string};
use std::path::{Path, PathBuf};

use crate::error as err;
use crate::objects as obj;

pub fn resolve_ref(ref_path: &Path, repo: &obj::Repo) -> Result<String, err::Error> {
    let data = read_to_string(repo.gitdir.join(ref_path))?;
    if "ref: " == &data[..5] {
        resolve_ref(&PathBuf::from(data[5..].trim()), repo)
    } else {
        return Ok(data.trim().to_owned());
    }
}

pub fn gather_refs(path: Option<&Path>, repo: &obj::Repo) -> Result<Vec<String>, err::Error> {
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
            let mut nested_refs = gather_refs(Some(rfs_path), repo)?;
            all_refs.append(&mut nested_refs);
        } else {
            // resolve_ref expects paths relative to .git/
            let clean_rf_path = rfs_path.strip_prefix(&repo.gitdir)?.to_owned();
            let resolved_ref = resolve_ref(&clean_rf_path, repo)?;
            if let Some(clean_path) = clean_rf_path.to_str() {
                all_refs.push(format!("{resolved_ref} {clean_path}\n"));
            } else {
                return Err(err::Error::PathToUtf8Conversion);
            };
        }
    }
    return Ok(all_refs);
}

#[cfg(test)]
mod refs_tests {
    use super::*;
    use crate::test_utils;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn resolve_ref_follows_indirect_refs_until_direct_ref() {
        let gitdir = test_utils::test_gitdir().unwrap();

        let foo_path = gitdir.path().join(".git/refs/heads/foo");
        let mut foo_ref = File::create(&foo_path).unwrap();
        writeln!(foo_ref, "ref: refs/heads/bar").unwrap();

        let direct_ref = "123shaABC";
        let mut bar_ref = File::create(gitdir.path().join(".git/refs/heads/bar")).unwrap();
        writeln!(bar_ref, "{}", &direct_ref).unwrap();

        let repo = obj::Repo::new(gitdir.path().to_path_buf()).unwrap();
        let resolved_ref = resolve_ref(&foo_path, &repo).unwrap();

        assert_eq!(direct_ref, resolved_ref);
    }
}
