use std::collections::HashSet;
use std::env;
use std::fs;
use std::process;

use dedup::types::FileEntry;

fn main() {
    if let Err(err) = run() {
        eprintln!("Error {}", err);
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "dup_size_check".into());

    let input_file = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("Usage: {} <input.json>", program);
            process::exit(1);
        }
    };

    let data = fs::read_to_string(&input_file)?;
    let groups: Vec<Vec<FileEntry>> = serde_json::from_str(&data)?;

    for (group_index, group) in groups.iter().enumerate() {
        if group.is_empty() {
            continue;
        }

        let checksum_set: HashSet<&str> =
            group.iter().map(|e| e.checksum.as_str()).collect();
        let size_set: HashSet<u64> =
            group.iter().map(|e| e.size).collect();

        if checksum_set.len() == 1 && size_set.len() > 1 {
            println!(
                "⚠️  Group {} has the same checksum, but different size:",
                group_index
            );

            for entry in group {
                println!(
                    "  path: {:<60} size: {:<10} sha256: {}",
                    entry.path,
                    entry.size,
                    entry.checksum
                );
            }
            println!();
        }
    }

    Ok(())
}

