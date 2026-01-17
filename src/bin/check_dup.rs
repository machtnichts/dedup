use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::process;

#[derive(Debug, Deserialize)]
struct Entry {
    path: String,
    size: u64,
    sha256: String,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Fehler: {}", err);
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Commandline-Argumente lesen
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "dup_size_check".into());

    let input_file = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("Usage: {} <input.json>", program);
            process::exit(1);
        }
    };

    // JSON-Datei einlesen
    let data = fs::read_to_string(&input_file)?;
    let groups: Vec<Vec<Entry>> = serde_json::from_str(&data)?;

    for (group_index, group) in groups.iter().enumerate() {
        if group.is_empty() {
            continue;
        }

        let sha_set: HashSet<&str> =
            group.iter().map(|e| e.sha256.as_str()).collect();
        let size_set: HashSet<u64> =
            group.iter().map(|e| e.size).collect();

        // gleiche Checksumme, aber unterschiedliche Größen
        if sha_set.len() == 1 && size_set.len() > 1 {
            println!(
                "⚠️  Gruppe {} hat gleiche SHA256, aber unterschiedliche Größen:",
                group_index
            );

            for entry in group {
                println!(
                    "  path: {:<60} size: {:<10} sha256: {}",
                    entry.path,
                    entry.size,
                    entry.sha256
                );
            }
            println!();
        }
    }

    Ok(())
}

