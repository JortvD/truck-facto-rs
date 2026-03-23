use std::collections::HashMap;
use crate::{git::AuthorInfo, tf::AuthorFiles};

/// Calculates the Gini coefficient to measure the inequality of authorship 
/// distribution among contributors.
pub fn calculate_gini(authors: &mut HashMap<AuthorInfo, AuthorFiles>) -> f64 {
    let mut x: Vec<f64> = authors.values().map(|files| files.files.len() as f64).collect();
    
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