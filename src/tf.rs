use std::collections::{HashMap, HashSet};
use crate::{git::AuthorInfo, doa::DoaFile};

const COVERAGE_THRESHOLD: f64 = 0.5;

/// Holds the files associated with an author and whether they are currently
/// included in the Truck Factor calculation.
pub struct AuthorFiles {
    pub files: Vec<DoaFile>,
    pub included: bool,
}

impl AuthorFiles {
    fn new() -> Self {
        Self { files: Vec::new(), included: true }
    }
}

fn get_authors_map(files: &[DoaFile]) -> HashMap<AuthorInfo, AuthorFiles> {
    let mut authors_map: HashMap<AuthorInfo, AuthorFiles> = HashMap::new();

    for file in files {
        for author in file.get_authors() {
            authors_map.entry(author)
                .or_insert_with(AuthorFiles::new)
                .files.push(file.clone());
        }
    }
    authors_map
}

fn get_coverage(authors_map: &HashMap<AuthorInfo, AuthorFiles>, total_files: usize) -> f64 {
    let included_files: HashSet<&String> = authors_map.values()
        .filter(|af| af.included)
        .flat_map(|af| af.files.iter().map(|f| &f.name))
        .collect();

    included_files.len() as f64 / total_files as f64
}

fn included_size(authors_map: &HashMap<AuthorInfo, AuthorFiles>) -> usize {
    authors_map.values().filter(|files| files.included).count()
}

fn exclude_largest(authors_map: &mut HashMap<AuthorInfo, AuthorFiles>) {
    let largest = authors_map.values_mut()
        .filter(|files| files.included)
        .max_by_key(|files| files.files.len());
        
    if let Some(item) = largest {
        item.included = false;
    }
}

/// Computes the Truck Factor: the minimal number of developers that have to be 
/// incapacitated to make a project lose more than 50% of its file coverage.
pub fn calculate_truck_factor(files: &[DoaFile]) -> (u64, HashMap<AuthorInfo, AuthorFiles>) {
    let mut authors_map = get_authors_map(files);
    let mut tf = 0;

    while included_size(&authors_map) > 0 {
        let coverage = get_coverage(&authors_map, files.len());
        if coverage < COVERAGE_THRESHOLD {
            return (tf, authors_map);
        }

        exclude_largest(&mut authors_map);
        tf += 1;
    }

    (0, authors_map)
}