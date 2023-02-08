use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;

use crate::error as err;
use crate::utils;

pub fn default_repo_config() -> &'static str {
    "[core]
       bare = false
       filemode = false
       repositoryformatversion = 0"
}

pub fn create_git_repo(path: &Path) -> Result<Option<String>, err::Error> {
    if utils::is_git_repo(path) {
        return Err(err::Error::GitRepoAlreadyExists);
    }
    File::create(path.join(".rusty-git-allowed"))?;

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

#[cfg(test)]
mod init_tests {
    use super::*;
    use crate::test_utils;
    use std::collections::HashMap;

    #[test]
    fn create_git_repo_succeeds_in_empty_dir() {
        let tempdir = test_utils::test_tempdir().unwrap();
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
    fn create_git_repo_errors_in_an_existing_git_dir() {
        let gitdir = test_utils::test_gitdir().unwrap();
        let gitdir_path = gitdir.path();
        assert!(utils::is_git_repo(gitdir_path));

        let create_git_repo_result = create_git_repo(gitdir_path);
        assert!(Err(err::Error::GitRepoAlreadyExists) == create_git_repo_result);
    }
}
