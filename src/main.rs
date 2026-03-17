use std::path::Path;
use std::env;
use std::process;

mod file;
mod git;
mod doa;
mod tf;
mod gini;

/// Entry point for the repository analysis tool.
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <path-to-repo>", args[0]);
        process::exit(1);
    }

    let repo_path_str = &args[1];
    let repo_path = Path::new(repo_path_str);

    if !repo_path.exists() {
        eprintln!("Error: The path '{}' does not exist.", repo_path_str);
        process::exit(1);
    }

    let total_start_time = std::time::Instant::now();
    let repo = git::Repo { path: repo_path };
    
    // 1. Fetch commits
    let start_time = std::time::Instant::now();
    let mut commits = git::get_commit_info(&repo);
    println!("Fetched {} commits in {:?}", commits.len(), start_time.elapsed());

    // 2. Populate file histories
    let start_time = std::time::Instant::now();
    git::populate_files_for_commits(&repo, &mut commits);
    println!("Populated files for commits in {:?}", start_time.elapsed());

    // 3. Merge aliases
    let start_time = std::time::Instant::now();
    let (mergers, authors) = git::merge_alias_authors(&mut commits);
    println!("Merged {} authors to {} get total authors in {:?}", mergers, authors, start_time.elapsed());

    // 4. Discover and filter files
    let start_time = std::time::Instant::now();
    let mut files = file::get_files_in_repo(&repo);
    println!("Found {} files in {:?}", files.len(), start_time.elapsed());

    let start_time = std::time::Instant::now();
    file::mark_vendored_files(&mut files, &repo);
    let vendored_files = files.iter().filter(|f| f.filtered).count();
    println!("Marked {} as vendored in {:?}", vendored_files, start_time.elapsed());

    // 5. Track file renames
    let start_time = std::time::Instant::now();
    let file_names: Vec<String> = files.iter().filter(|f| !f.filtered).map(|f| f.name.clone()).collect();
    git::assign_recent_names(&file_names, &mut commits);
    println!("Assigned recent names for {} files in {:?}", file_names.len(), start_time.elapsed());

    // 6. Calculate Degree of Authorship (DOA)
    let start_time = std::time::Instant::now();
    let doa_files = doa::prepare_for_doa(&file_names, &commits);
    println!("Prepared {} files for DOA calculation in {:?}", doa_files.len(), start_time.elapsed());

    // 7. Calculate Truck Factor
    let (tf, authors) = tf::calculate_truck_factor(&doa_files);

    // 8. Calculate Gini Coefficient
    let gini = gini::calculate_gini(&doa_files);

    println!("TF: {}, Gini: {:.4} in {:?}", tf, gini, total_start_time.elapsed());
    for (author, files) in authors {
        println!("{} - '{}' <{}> ({})", 
            if files.included { " " } else { "x" }, 
            author.name, author.email, files.files.len()
        );
    }
}