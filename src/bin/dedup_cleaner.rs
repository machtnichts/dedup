use anyhow::{Context, Result};
use serde::Deserialize;
use std::{
    collections::HashSet,
    env,
    fs,
    io::{self, Write},
    path::PathBuf,
};

#[derive(Debug, Deserialize)]
struct FileEntry {
    path: String,
//    size: u64,
//    sha256: String,
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: dedup_cleaner <duplicates.json> [--dry-run]");
        return Ok(());
    }

    let json_path = &args[1];
    let dry_run = args.get(2).map(|s| s == "--dry-run").unwrap_or(false);

    let data = fs::read_to_string(json_path)
        .with_context(|| format!("Failed to read {}", json_path))?;

    let groups: Vec<Vec<FileEntry>> =
        serde_json::from_str(&data).context("Invalid JSON format")?;

    println!("Loaded {} duplicate groups\n", groups.len());
    if dry_run {
        println!("*** DRY-RUN MODE: no files will be deleted ***\n");
    }

    let mut preferred_dirs: HashSet<PathBuf> = HashSet::new();
    let mut processed_groups = vec![false; groups.len()];

    for group_index in 0..groups.len() {
        if processed_groups[group_index] {
            continue;
        }

        let group = &groups[group_index];
        if group.is_empty() {
            processed_groups[group_index] = true;
            continue;
        }

        // Collect all parent directories in this group
        let mut dirs_in_group: HashSet<PathBuf> = HashSet::new();
        for file in group {
            if let Some(parent) = PathBuf::from(&file.path).parent() {
                dirs_in_group.insert(parent.to_path_buf());
            }
        }

        // If all files are in the same directory, skip this group
        if dirs_in_group.len() == 1 {
            println!(
                "Skipping group #{}: all files in same directory ({})\n",
                group_index + 1,
                dirs_in_group.iter().next().unwrap().display()
            );
            processed_groups[group_index] = true;
            continue;
        }

        println!(
            "Duplicate group #{} ({} files, multiple directories)",
            group_index + 1,
            group.len()
        );

        // Check for already preferred dirs
        let mut matching_dirs = Vec::new();
        for dir in &dirs_in_group {
            if preferred_dirs.contains(dir) {
                matching_dirs.push(dir.clone());
            }
        }

        let keep_dir_opt: Option<PathBuf> = if matching_dirs.len() == 1 {
            let dir = matching_dirs[0].clone();
            println!(
                "Using preferred directory automatically:\n  {}\n",
                dir.display()
            );
            Some(dir)
        } else {
            // Ask user
            for (i, file) in group.iter().enumerate() {
                println!("[{}] {}", i + 1, file.path);
            }

            loop {
                let choice = ask_choice(group.len())?;
                if choice == 0 {
                    println!("Group skipped.\n");
                    break None;
                } else if choice == usize::MAX {
                    println!("Cancel requested. Exiting.");
                    return Ok(());
                } else {
                    let chosen_path = PathBuf::from(&group[choice - 1].path);
                    let dir = chosen_path
                        .parent()
                        .context("Failed to determine parent directory")?
                        .to_path_buf();

                    println!(
                        "Selected preferred directory:\n  {}\n",
                        dir.display()
                    );
                    preferred_dirs.insert(dir.clone());
                    break Some(dir);
                }
            }
        };

        if let Some(keep_dir) = keep_dir_opt {
            // Process all groups containing files in this directory
            for (idx, grp) in groups.iter().enumerate() {
                if processed_groups[idx] {
                    continue;
                }

                let has_in_keep_dir = grp.iter().any(|f| {
                    let path = PathBuf::from(&f.path);
                    path.starts_with(&keep_dir)
                });

                if has_in_keep_dir {
                    processed_groups[idx] = true;
                    let mut deleted = 0usize;
                    for file in grp {
                        let file_path = PathBuf::from(&file.path);
                        if file_path.starts_with(&keep_dir) {
                            continue;
                        }

                        if dry_run {
                            println!("Would delete: {}", file_path.display());
                            deleted += 1;
                        } else {
                            match fs::remove_file(&file_path) {
                                Ok(_) => {
                                    println!("Deleted: {}", file_path.display());
                                    deleted += 1;
                                }
                                Err(e) => {
                                    eprintln!("Failed to delete {}: {}", file_path.display(), e);
                                }
                            }
                        }
                    }
                    if deleted > 0 || dry_run {
                        println!(
                            "Group #{} finished, {} file(s) {}.\n",
                            idx + 1,
                            deleted,
                            if dry_run { "would be deleted" } else { "deleted" }
                        );
                    }
                }
            }
        }
    }

    println!("All duplicate groups processed.");
    Ok(())
}

fn ask_choice(max: usize) -> Result<usize> {
    loop {
        print!(
            "Which file should define the preferred directory? Enter 1-{}, s=skip group, c=cancel all: ",
            max
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        match input.as_str() {
            "s" => return Ok(0),
            "c" => return Ok(usize::MAX),
            _ => {
                if let Ok(num) = input.parse::<usize>() {
                    if num >= 1 && num <= max {
                        return Ok(num);
                    }
                }
            }
        }

        println!("Invalid input, please try again.\n");
    }
}
