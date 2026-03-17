use std::process::Command;
use linguist::is_vendored;

use crate::git::Repo;

/// Represents a file within the repository and its vendored status.
pub struct File {
    pub name: String,
    pub filtered: bool,
}

impl File {
    pub fn new(name: String) -> Self {
        Self { name, filtered: false }
    }
}

/// Lists all files currently tracked in the Git repository.
pub fn get_files_in_repo(repo: &Repo) -> Vec<File> {
    let output = Command::new("git")
        .arg("ls-files")
        .current_dir(repo.path)
        .output()
        .expect("Failed to run git ls-files");

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| File::new(line.to_string()))
        .collect()
}

/// Updates the `filtered` flag for files that are identified as vendored dependencies.
pub fn mark_vendored_files(files: &mut [File], repo: &Repo) {
    for file in files.iter_mut() {
        let file_path = repo.path.join(&file.name);
        if let Ok(result) = is_vendored(&file_path) {
            file.filtered = result;
        }
    }
}