use std::{collections::{HashMap, HashSet}, process::Command};
use std::path::Path;

/// Represents a local Git repository.
pub struct Repo<'a> {
    pub path: &'a Path
}

const MIN_SIZE: usize = 3;
const MAX_DISTANCE: usize = 1;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct AuthorInfo {
    pub name: String,
    pub email: String
}

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub hash: String,
    pub author: AuthorInfo,
    pub author_date: u64,
    pub committer: AuthorInfo,
    pub committer_date: u64,
    pub message: String,
    pub files: Vec<CommitFileInfo>,
}

impl CommitInfo {
    /// Returns the main author of the commit, falling back to the committer.
    pub fn get_main_author(&self) -> Option<&AuthorInfo> {
        if !self.author.email.is_empty() || !self.author.name.is_empty() {
            Some(&self.author)
        } else if !self.committer.email.is_empty() || !self.committer.name.is_empty() {
            Some(&self.committer)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
    Renamed(String), // Contains the old name
}

#[derive(Debug, Clone)]
pub struct CommitFileInfo {
    pub file_name: String,
    pub change_type: ChangeType,
    pub recent_file_name: Option<String>,
}

/// Parses the git log to extract metadata for all commits.
pub fn get_commit_info(repo: &Repo) -> HashMap<String, CommitInfo> {
    let output = Command::new("git")
        .args(["log", "--pretty=format:\"%H-;-%aN-;-%aE-;-%at-;-%cN-;-%cE-;-%ct-;-%f\""])
        .current_dir(repo.path)
        .output()
        .expect("Failed to run git log");

    String::from_utf8_lossy(&output.stdout)
        .split('\n')
        .filter_map(parse_log_entry)
        .map(|commit| (commit.hash.clone(), commit))
        .collect()
}

/// Helper function to parse a single `git log` line into a `CommitInfo`.
fn parse_log_entry(entry: &str) -> Option<CommitInfo> {
    let parts: Vec<&str> = entry.trim_matches('"').split("-;-").collect();
    if parts.len() != 8 {
        return None;
    }

    Some(CommitInfo {
        hash: parts[0].to_string(),
        author: AuthorInfo {
            name: parts[1].to_ascii_lowercase(),
            email: parts[2].to_ascii_lowercase(),
        },
        author_date: parts[3].parse().unwrap_or(0),
        committer: AuthorInfo {
            name: parts[4].to_ascii_lowercase(),
            email: parts[5].to_ascii_lowercase(),
        },
        committer_date: parts[6].parse().unwrap_or(0),
        message: parts[7].to_string(),
        files: Vec::new(),
    })
}

/// Populates the files modified in each commit and tracks their change type.
pub fn populate_files_for_commits(repo: &Repo, commits: &mut HashMap<String, CommitInfo>) {
    Command::new("git")
        .args(["config", "diff.renameLimit", "999999"])
        .current_dir(repo.path)
        .output()
        .expect("Failed to set git diff renameLimit");

    let output = Command::new("git")
        .args(["log", "--name-status", "--pretty=format:commit %H", "--find-renames"])
        .current_dir(repo.path)
        .output()
        .expect("Failed to run git log with name-status");

    let mut current_commit_hash: Option<String> = None;

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if let Some(hash) = line.strip_prefix("commit ") {
            current_commit_hash = Some(hash.to_string());
            continue;
        }

        if let Some(commit_hash) = &current_commit_hash {
            if let Some(commit_info) = commits.get_mut(commit_hash) {
                if let Some(file_info) = parse_name_status_line(line) {
                    commit_info.files.push(file_info);
                }
            }
        }
    }
}

/// Helper function to parse a git `--name-status` line into `CommitFileInfo`.
fn parse_name_status_line(line: &str) -> Option<CommitFileInfo> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let change_type = match parts[0] {
        "A" => ChangeType::Added,
        "M" => ChangeType::Modified,
        "D" => ChangeType::Deleted,
        s if s.starts_with('R') && parts.len() == 3 => ChangeType::Renamed(parts[1].to_string()),
        _ => return None,
    };

    Some(CommitFileInfo {
        file_name: parts.last().unwrap().to_string(),
        change_type,
        recent_file_name: None,
    })
}

/// Traces files backwards through time to assign them their modern filenames,
/// accounting for historical renames.
pub fn assign_recent_names(files: &[String], commits: &mut HashMap<String, CommitInfo>) {
    let mut active_traces: HashMap<String, String> = files.iter()
        .map(|f| (f.clone(), f.clone()))
        .collect();

    let mut sorted_commits: Vec<&mut CommitInfo> = commits.values_mut().collect();
    sorted_commits.sort_unstable_by(|a, b| b.committer_date.cmp(&a.committer_date));

    let mut pending_removals: Vec<&str> = Vec::new();
    let mut pending_insertions: Vec<(&String, String)> = Vec::new();

    for commit in sorted_commits {
        if active_traces.is_empty() {
            break; 
        }

        pending_removals.clear();
        pending_insertions.clear();

        for file_info in &mut commit.files {
            let current_name = &file_info.file_name;

            if let Some(recent_name) = active_traces.get(current_name) {
                file_info.recent_file_name = Some(recent_name.clone());

                match &file_info.change_type {
                    ChangeType::Renamed(old_name) => {
                        pending_removals.push(current_name.as_str());
                        pending_insertions.push((old_name, recent_name.clone()));
                    }
                    ChangeType::Added => {
                        pending_removals.push(current_name.as_str());
                    }
                    _ => {}
                }
            }
        }

        for r in &pending_removals {
            active_traces.remove(*r);
        }
        for (k, v) in pending_insertions.drain(..) {
            active_traces.insert(k.clone(), v);
        }
    }
}

// -----------------------------------------------------------------------------
// Author Merging Logic
// -----------------------------------------------------------------------------

struct DisjointSet {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl DisjointSet {
    fn new(size: usize) -> Self {
        Self {
            parent: (0..size).collect(),
            rank: vec![0; size],
        }
    }

    fn find(&mut self, i: usize) -> usize {
        if self.parent[i] == i {
            i
        } else {
            let root = self.find(self.parent[i]);
            self.parent[i] = root;
            root
        }
    }

    fn union(&mut self, i: usize, j: usize) -> bool {
        let root_i = self.find(i);
        let root_j = self.find(j);
        
        if root_i != root_j {
            match self.rank[root_i].cmp(&self.rank[root_j]) {
                std::cmp::Ordering::Less => self.parent[root_i] = root_j,
                std::cmp::Ordering::Greater => self.parent[root_j] = root_i,
                std::cmp::Ordering::Equal => {
                    self.parent[root_j] = root_i;
                    self.rank[root_i] += 1;
                }
            }
            true
        } else {
            false
        }
    }
}

fn is_similar(a: &AuthorInfo, b: &AuthorInfo) -> bool {
    let exact_name = !a.name.is_empty() && a.name == b.name;
    if exact_name { return true; }

    let exact_email = !a.email.is_empty() && a.email == b.email;
    if exact_email { return true; }

    let similar_name = a.name.len() >= MIN_SIZE && b.name.len() >= MIN_SIZE
        && a.name.len().abs_diff(b.name.len()) <= MAX_DISTANCE
        && levenshtein::levenshtein(&a.name, &b.name) <= MAX_DISTANCE;

    if similar_name { return true; }

    let similar_email = a.email.len() >= MIN_SIZE && b.email.len() >= MIN_SIZE
        && a.email.len().abs_diff(b.email.len()) <= MAX_DISTANCE
        && levenshtein::levenshtein(&a.email, &b.email) <= MAX_DISTANCE;

    similar_email
}

fn reduce_overlapping_strings(inputs: impl Iterator<Item = String>) -> Vec<String> {
    let mut inputs: Vec<String> = inputs.filter(|s| !s.is_empty()).collect();
    
    inputs.sort_unstable_by(|a, b| b.len().cmp(&a.len()));

    let mut unique: Vec<String> = Vec::new();
    for item in inputs {
        if !unique.iter().any(|existing| existing.contains(&item)) {
            unique.push(item);
        }
    }
    
    unique.sort_unstable();
    unique
}

fn merge_author_cluster(cluster: &[&AuthorInfo]) -> AuthorInfo {
    let unique_names = reduce_overlapping_strings(cluster.iter().map(|a| a.name.clone()));
    let unique_emails = reduce_overlapping_strings(cluster.iter().map(|a| a.email.clone()));

    AuthorInfo {
        name: unique_names.join(" && "),
        email: unique_emails.join(" && "),
    }
}

fn find_unique_authors(commits: &HashMap<String, CommitInfo>) -> (HashMap<AuthorInfo, AuthorInfo>, usize, usize) {
    let unique_authors: Vec<AuthorInfo> = commits.values()
        .filter_map(|c| c.get_main_author().cloned())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let n = unique_authors.len();
    let mut ds = DisjointSet::new(n);
    let mut mergers = 0;

    let mut name_map: HashMap<&str, Vec<usize>> = HashMap::new();
    let mut email_map: HashMap<&str, Vec<usize>> = HashMap::new();

    for (i, author) in unique_authors.iter().enumerate() {
        if !author.name.is_empty() {
            name_map.entry(&author.name).or_default().push(i);
        }
        if !author.email.is_empty() {
            email_map.entry(&author.email).or_default().push(i);
        }
    }

    for indices in name_map.values().chain(email_map.values()) {
        let first = indices[0];
        for &idx in indices.iter().skip(1) {
            if ds.union(first, idx) {
                mergers += 1;
            }
        }
    }

    for i in 0..n {
        for j in (i + 1)..n {
            if ds.find(i) == ds.find(j) {
                continue;
            }

            if is_similar(&unique_authors[i], &unique_authors[j]) {
                if ds.union(i, j) {
                    mergers += 1;
                }
            }
        }
    }

    let mut clusters: HashMap<usize, Vec<&AuthorInfo>> = HashMap::new();
    for i in 0..n {
        let root = ds.find(i);
        clusters.entry(root).or_default().push(&unique_authors[i]);
    }

    let mut author_map: HashMap<AuthorInfo, AuthorInfo> = HashMap::new();
    for cluster in clusters.values() {
        let merged_author = merge_author_cluster(cluster);
        for author in cluster {
            author_map.insert((*author).clone(), merged_author.clone());
        }
    }

    (author_map, mergers, n-mergers)
}

pub fn merge_alias_authors(commits: &mut HashMap<String, CommitInfo>) -> (usize, usize) {
    let (author_map, mergers, authors) = find_unique_authors(commits);

    for commit in commits.values_mut() {
        if let Some(author) = commit.get_main_author() {
            if let Some(unique_author) = author_map.get(author) {
                commit.author = unique_author.clone();
            }
        }
    }
    
    (mergers, authors)
}