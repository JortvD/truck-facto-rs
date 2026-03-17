use std::collections::HashMap;
use crate::{git::AuthorInfo, doa::DoaFile};

/// Calculates the Gini coefficient to measure the inequality of authorship 
/// distribution among contributors.
pub fn calculate_gini(files: &[DoaFile]) -> f64 {
    let mut authors_map: HashMap<AuthorInfo, Vec<DoaFile>> = HashMap::new();

    for file in files {
        for author in file.get_authors() {
            authors_map.entry(author).or_default().push(file.clone());
        }
    }

    let mut x: Vec<f64> = authors_map.values().map(|files| files.len() as f64).collect();
    
    // Sort ascending
    x.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    
    let n = x.len() as f64;
    if n == 0.0 {
        return 0.0;
    }

    let sum_x: f64 = x.iter().sum();
    let sum_i_x: f64 = x.iter().enumerate().map(|(i, &xi)| (i as f64 + 1.0) * xi).sum();

    (2.0 * sum_i_x) / (n * sum_x) - (n + 1.0) / n
}