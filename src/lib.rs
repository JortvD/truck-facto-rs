//! # truck-facto-rs
//!
//! `truck-facto-rs` is a high-performance Rust library for analyzing Git repositories 
//! to determine their **Truck Factor** and **Gini Coefficient**.
//!
//! ## Example Usage
//!
//! ```no_run
//! use std::path::Path;
//! use truck_facto_rs::{git, file, doa, tf, gini};
//!
//! // 1. Point the analyzer to a local Git repository
//! let repo_path = Path::new("./my-rust-project");
//! let repo = git::Repo { path: repo_path };
//!
//! // 2. Fetch all commits and their file modification histories
//! let mut commits = git::get_commit_info(&repo);
//! git::populate_files_for_commits(&repo, &mut commits);
//!
//! // 3. Deduplicate authors (merge aliases, typos, and secondary emails)
//! git::merge_alias_authors(&mut commits);
//!
//! // 4. Discover current files and filter out vendored dependencies
//! let mut files = file::get_files_in_repo(&repo);
//! file::mark_vendored_files(&mut files, &repo);
//! 
//! let active_files: Vec<String> = files.into_iter()
//!     .filter(|f| !f.filtered)
//!     .map(|f| f.name)
//!     .collect();
//!
//! // 5. Trace historical file renames
//! git::assign_recent_names(&active_files, &mut commits);
//!
//! // 6. Calculate Degree of Authorship (DOA)
//! let doa_files = doa::prepare_for_doa(&active_files, &commits);
//!
//! // 7. Calculate Truck Factor and Gini Coefficient
//! let (truck_factor, _author_coverage) = tf::calculate_truck_factor(&doa_files);
//! let gini_coeff = gini::calculate_gini(&doa_files);
//! 
//! println!("Truck Factor: {}", truck_factor);
//! println!("Gini Coefficient: {:.4}", gini_coeff);
//! ```

/// Commit parsing, history traversal, and author deduplication logic.
pub mod git;

/// Degree of Authorship (DOA) heuristics and calculations.
pub mod doa;

/// File discovery and vendored dependency filtering.
pub mod file;

/// Mathematical calculation of the Gini coefficient for code contributions.
pub mod gini;

/// Truck factor calculation and author coverage mapping.
pub mod tf;