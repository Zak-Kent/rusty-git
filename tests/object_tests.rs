mod tests {
    use tempfile::{tempdir, TempDir};
    use std::fs::{create_dir, File};
    use std::io::{Error};

    fn test_git_dir () -> Result<TempDir, Error> {
        let dir = tempdir()?;
        create_dir(dir.path().join(".git"))?;
        File::create(dir.path().join("config"))?;
        return Ok(dir)
    }

    #[test]
    fn git_repo_setup_test () {
        // unwrap will panic here if dir setup fails
        let worktree = test_git_dir().unwrap();
        let gitdir = worktree.path().join(".git");
        let gitconf = worktree.path().join("config");

        assert!(gitdir.exists());
        assert!(gitconf.exists());
    }
}
