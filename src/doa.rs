use std::{collections::HashMap, f64::consts::E};
use crate::git::{AuthorInfo, ChangeType, CommitInfo};

const MIN_NORM_DOA: f64 = 0.75;
const MIN_ABS_DOA: f64 = 3.293;

/// Represents a file tracked for Degree of Authorship calculations.
#[derive(Debug, Clone)]
pub struct DoaFile {
    pub name: String,
    pub authorships: Vec<Authorship>,
    pub num_changes: usize,
}

impl DoaFile {
    pub fn new(name: String) -> Self {
        Self { name, authorships: Vec::new(), num_changes: 0 }
    }

    /// Determines the significant authors of a file based on DOA thresholds.
    pub fn get_authors(&self) -> Vec<AuthorInfo> {
        let max_doa = self.authorships.iter()
            .map(|a| a.calculate_doa(self.num_changes))
            .fold(0.0_f64, f64::max);

        self.authorships.iter().filter_map(|a| {
            let doa = a.calculate_doa(self.num_changes);
            if doa / max_doa >= MIN_NORM_DOA && (doa >= MIN_ABS_DOA || a.added) {
                Some(a.author.clone())
            } else {
                None
            }
        }).collect()
    }

    fn find_mut_authorship(&mut self, author: &AuthorInfo) -> Option<&mut Authorship> {
        self.authorships.iter_mut().find(|a| a.author == *author)
    }

    pub fn insert_added(&mut self, author: AuthorInfo) {
        if let Some(authorship) = self.find_mut_authorship(&author) {
            authorship.added = true;
        } else {
            self.authorships.push(Authorship { author, added: true, num_deliveries: 0 });
        }
    }

    pub fn insert_delivery(&mut self, author: AuthorInfo) {
        if let Some(authorship) = self.find_mut_authorship(&author) {
            authorship.num_deliveries += 1;
        } else {
            self.authorships.push(Authorship { author, added: false, num_deliveries: 1 });
        }
        self.num_changes += 1;
    }
}

/// Tracks an author's contribution scale to a specific file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Authorship {
    pub author: AuthorInfo,
    pub added: bool,
    pub num_deliveries: usize,
}

impl Authorship {
    /// Calculates the standard Degree of Authorship (DOA) score.
    pub fn calculate_doa(&self, file_deliveries: usize) -> f64 {
        let fa = 1.098 * (if self.added { 1.0 } else { 0.0 });
        let dl = 0.164 * self.num_deliveries as f64;
        let ac = 0.321 * (1.0 + (file_deliveries - self.num_deliveries) as f64).log(E);
        
        3.293 + fa + dl - ac
    }
}

/// Processes commits to construct DOA files with aggregated delivery metadata.
pub fn prepare_for_doa(files: &[String], commits: &HashMap<String, CommitInfo>) -> Vec<DoaFile> {
    let mut doa_map: HashMap<&str, DoaFile> = HashMap::with_capacity(files.len());
    for file_name in files {
        doa_map.insert(file_name.as_str(), DoaFile::new(file_name.clone()));
    }

    for commit in commits.values() {
        let author = match commit.get_main_author() {
            Some(a) => a,
            None => continue,
        };

        for file_info in &commit.files {
            if let Some(recent_name) = &file_info.recent_file_name {
                if let Some(doa_file) = doa_map.get_mut(recent_name.as_str()) {
                    match file_info.change_type {
                        ChangeType::Added => doa_file.insert_added(author.clone()),
                        ChangeType::Modified | ChangeType::Renamed(_) => {
                            doa_file.insert_delivery(author.clone())
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    doa_map.into_values().collect()
}