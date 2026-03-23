use std::collections::HashMap;
use crate::git::{AuthorInfo, ChangeType, CommitInfo};

const MIN_NORM_DOA: f64 = 0.75;
const MIN_ABS_DOA: f64 = 3.293;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Delivery {
    pub commit: String,

    #[cfg(feature = "decay")]
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Delivery {
    #[cfg(feature = "decay")]
    fn calculate_decay(&self, decay_days: f64, to: chrono::DateTime<chrono::Utc>) -> f64 {
        if to < self.timestamp {
            1.0
        } else {
            let elapsed = (to - self.timestamp).num_days() as f64 / (decay_days);
            (-elapsed).exp()
        }
    }
}

#[cfg(feature = "decay")]
fn calculate_delivery_decay(deliveries: &Vec<Delivery>, decay_days: f64, time: chrono::DateTime<chrono::Utc>) -> f64 {
    deliveries.iter().map(|d| d.calculate_decay(decay_days, time)).sum()
}

/// Represents a file tracked for Degree of Authorship calculations.
#[derive(Debug, Clone)]
pub struct DoaFile {
    pub name: String,
    pub authorships: Vec<Authorship>,
    pub deliveries: Vec<Delivery>,
}

impl DoaFile {
    pub fn new(name: String) -> Self {
        Self { name, authorships: Vec::new(), deliveries: Vec::new() }
    }

    /// Determines the significant authors of a file based on DOA thresholds.
    pub fn get_authors(&self) -> Vec<AuthorInfo> {
        let max_doa = self.authorships.iter()
            .map(|a| a.calculate_doa(&self.deliveries))
            .fold(0.0_f64, f64::max);

        self.authorships.iter().filter_map(|a| {
            let doa = a.calculate_doa(&self.deliveries);
            if doa / max_doa >= MIN_NORM_DOA && (doa >= MIN_ABS_DOA || a.added) {
                Some(a.author.clone())
            } else {
                None
            }
        }).collect()
    }

    #[cfg(feature = "decay")]
    pub fn get_decay_authors(&self, decay_days: f64, time: chrono::DateTime<chrono::Utc>) -> Vec<AuthorInfo> {
        let max_doa = self.authorships.iter()
            .map(|a| a.calculate_decay_doa(&self.deliveries, decay_days, time))
            .fold(0.0_f64, f64::max);

        self.authorships.iter().filter_map(|a| {
            let doa = a.calculate_decay_doa(&self.deliveries, decay_days, time);
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

    pub fn insert_added(&mut self, author: AuthorInfo, timestamp: chrono::DateTime<chrono::Utc>) {
        if let Some(authorship) = self.find_mut_authorship(&author) {
            authorship.added = true;
        } else {
            self.authorships.push(Authorship { 
                author, 
                added: true, 
                deliveries: Vec::new(),

                #[cfg(feature = "decay")]
                added_timestamp: Some(timestamp),
            });
        }
    }

    pub fn insert_delivery(&mut self, author: AuthorInfo, delivery: Delivery) {
        if let Some(authorship) = self.find_mut_authorship(&author) {
            authorship.deliveries.push(delivery.clone());
        } else {
            self.authorships.push(Authorship { 
                author, 
                added: false, 
                deliveries: vec![delivery.clone()], 

                #[cfg(feature = "decay")]
                added_timestamp: None,
            });
        }
        self.deliveries.push(delivery);
    }
}

/// Tracks an author's contribution scale to a specific file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Authorship {
    pub author: AuthorInfo,
    pub added: bool,
    pub deliveries: Vec<Delivery>,

    #[cfg(feature = "decay")]
    pub added_timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

impl Authorship {
    /// Calculates the standard Degree of Authorship (DOA) score.
    pub fn calculate_doa(&self, file_deliveries: &Vec<Delivery>) -> f64 {
        let fa = 1.098 * (if self.added { 1.0 } else { 0.0 });
        let dl = 0.164 * self.deliveries.len() as f64;
        let ac = 0.321 * (1.0 + (file_deliveries.len() - self.deliveries.len()) as f64).ln();
        
        3.293 + fa + dl - ac
    }

    #[cfg(feature = "decay")]
    /// Calculates a time-decayed Degree of Authorship (DOA) score.
    /// The decay factor reduces the influence of older contributions based on the specified decay days.
    pub fn calculate_decay_doa(&self, file_deliveries: &Vec<Delivery>, decay_days: f64, time: chrono::DateTime<chrono::Utc>) -> f64 {
        let fa = 1.098 * self.calculate_decay_added(decay_days, time);
        let author_deliveries_decay = calculate_delivery_decay(&self.deliveries, decay_days, time);
        let dl = 0.164 * author_deliveries_decay;
        let all_deliveries_decay = calculate_delivery_decay(file_deliveries, decay_days, time);
        let ac = 0.321 * (1.0 + all_deliveries_decay - author_deliveries_decay).ln();

        3.293 + fa + dl - ac
    }

    #[cfg(feature = "decay")]
    fn calculate_decay_added(&self, decay_days: f64, time: chrono::DateTime<chrono::Utc>) -> f64 {
        if self.added && let Some(added_time) = self.added_timestamp {
            (-((time - added_time).num_days() as f64 / decay_days)).exp()
        } else {
            0.0
        }
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

        #[cfg(feature = "decay")]
        let timestamp = chrono::DateTime::<chrono::Utc>::from_timestamp(commit.committer_date as i64, 0).unwrap_or_else(|| chrono::Utc::now());

        let delivery = Delivery { 
            commit: commit.hash.clone(), 
            
            #[cfg(feature = "decay")]
            timestamp
        };
        

        for file_info in &commit.files {
            if let Some(recent_name) = &file_info.recent_file_name {
                if let Some(doa_file) = doa_map.get_mut(recent_name.as_str()) {
                    match file_info.change_type {
                        ChangeType::Added => doa_file.insert_added(author.clone(), timestamp),
                        ChangeType::Modified | ChangeType::Renamed(_) => {
                            doa_file.insert_delivery(author.clone(), delivery.clone());
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    doa_map.into_values().collect()
}