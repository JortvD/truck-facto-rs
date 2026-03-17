# truck-facto-rs

**truck-facto-rs** is a high-performance Rust library for analyzing Git repositories to determine their **Truck Factor** (Avelino et al., 2016) and **Gini Coefficient**.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
truck-facto-rs = "0.1.0"
```

## CLI usage

After building you can run the default settings on a local repository as:
```bash
truck-factor-rs <local_repository>
```

## Example

Below is a complete example of how to use the `truck-facto-rs` pipeline to analyze a local Git repository.

```rust
use std::path::Path;
use truck_facto_rs::{
    repo::Repo,
    git, file, doa, tf, gini,
};

fn main() {
    // 1. Point the analyzer to a local Git repository
    let repo_path = Path::new("./my-rust-project");
    let repo = Repo { path: repo_path };

    // 2. Fetch all commits and their file modification histories
    let mut commits = git::get_commit_info(&repo);
    git::populate_files_for_commits(&repo, &mut commits);

    // 3. Deduplicate authors (merge aliases, typos, and secondary emails)
    let merged_count = git::merge_alias_authors(&mut commits);
    println!("Merged {} duplicate author profiles.", merged_count.0);

    // 4. Discover current files and filter out vendored/third-party dependencies
    let mut files = file::get_files_in_repo(&repo);
    file::mark_vendored_files(&mut files, &repo);
    
    // Extract only the actively tracked, non-vendored files
    let active_files: Vec<String> = files.into_iter()
        .filter(|f| !f.filtered)
        .map(|f| f.name)
        .collect();

    // 5. Trace historical file renames to associate old commits with modern filenames
    git::assign_recent_names(&active_files, &mut commits);

    // 6. Check who authored each file
    let doa_files = doa::prepare_for_doa(&active_files, &commits);

    // 7. Calculate the final Truck Factor
    let (truck_factor, author_coverage) = tf::calculate_truck_factor(&doa_files);
    
    println!("=== Final Report ===");
    println!("Truck Factor: {}", truck_factor);
    
    // 8. Calculate the Gini Coefficient
    let gini_coeff = gini::calculate_gini(&doa_files);
    println!("Gini Coefficient: {:.4}", gini_coeff);

    // Optional: Print the core authors and their file ownership count
    println!("\nCore Contributors:");
    for (author, ownership) in author_coverage {
        if ownership.included {
            println!("- {} <{}>: {} files", author.name, author.email, ownership.files.len());
        }
    }
}
```

## Performance

On a HP ZBook Studio G5 with a Intel® Core™ i7-9750H and 32GB running Ubuntu 25.10.

- `linux/linux` at `2d1373e4246d` has truck factor: 344, gini coeff: 0.7841 in 5m2.919s.
- `freeCodeCamp/freeCodeCamp` at `e66bf09dce` has truck factor: 6, gini coeff: 0.9205 in 14.217s.
- `facebook/react` at `3f0b9e61c4` has truck factor: 3, gini coeff: 0.8931 in 2.127s.
- `vuejs/vue` at `9e887079` has truck factor: 1, gini coeff: 0.8892 in 0.286s.
- `tensorflow/tensorflow` at `4c6c1dd02c2` has truck factor: 17, gini coeff: 0.8535 in 19.423s.
- `microsoft/vscode` at `a320ebfa541` has truck factor: 8, gini coeff: 0.8742 in 12.083s.
- `aserg-ufmg/Truck-Factor` at `614600e` has truck factor: 2, gini coeff: 0.6104 in 0.094s.

## Based on
Guilherme Avelino, Leonardo Passos, Andre Hora, Marco Tulio Valente. A Novel Approach for Estimating Truck Factors. In 24th International Conference on Program Comprehension (ICPC), pages 1-10, 2016.