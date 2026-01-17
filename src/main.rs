use anyhow::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    cmp::Reverse,
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::task;
use walkdir::WalkDir;

#[derive(Serialize, Debug, Clone)]
struct FileEntry {
    path: String,
    size: u64,
    sha256: String,
}

struct Stats {
    files: AtomicU64,
    bytes: AtomicU64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let root = std::env::args()
        .nth(1)
        .context("Usage: dirhash <directory>")?;

    println!("Processing folder: {}\n", root);

    let stats = Arc::new(Stats {
        files: AtomicU64::new(0),
        bytes: AtomicU64::new(0),
    });

    let mut tasks = Vec::new();

    for entry in WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let path = entry.path().to_path_buf();
            let stats = stats.clone();

            tasks.push(task::spawn_blocking(move || {
                process_file(path, stats)
            }));
        }
    }

    let mut files = Vec::new();

    for task in tasks {
        match task.await {
            Ok(Ok(entry)) => files.push(entry),
            Ok(Err(e)) => eprintln!("Error processing file: {}", e),
            Err(e) => eprintln!("Task failed: {}", e),
        }
    }

    // ---- sort all files by path
    files.sort_by(|a, b| a.path.cmp(&b.path));
    write_json("all_files.json", &files)?;

    // ---- find duplicates
    let mut map: HashMap<(u64, String), Vec<FileEntry>> = HashMap::new();

    for f in &files {
        map.entry((f.size, f.sha256.clone()))
            .or_default()
            .push(f.clone());
    }

    let mut duplicates: Vec<Vec<FileEntry>> = map
        .into_values()
        .filter(|group| group.len() > 1)
        .collect();

    // sort duplicate groups by file size
    duplicates.sort_by_key(|group| Reverse(group[0].size));
    write_json("duplicates.json", &duplicates)?;

    // ---- statistics
    let total_files = stats.files.load(Ordering::Relaxed);
    let total_bytes = stats.bytes.load(Ordering::Relaxed);

    let mut duplicate_files = 0u64;
    let mut potential_savings = 0u64;

    for group in &duplicates {
        let size = group[0].size;
        let count = group.len() as u64;

        duplicate_files += count;
        potential_savings += size * (count - 1); // keep one
    }

    println!("\n=== Statistics ===");
    println!("Total files           : {}", total_files);
    println!(
        "Total data processed  : {:.2} MB",
        total_bytes as f64 / 1_048_576.0
    );
    println!("Duplicate files       : {}", duplicate_files);
    println!("Duplicate groups      : {}", duplicates.len());
    println!(
        "Potential savings     : {:.2} MB",
        potential_savings as f64 / 1_048_576.0
    );

    println!("\nOutput written to:");
    println!("  all_files.json");
    println!("  duplicates.json");

    Ok(())
}

fn process_file(path: PathBuf, stats: Arc<Stats>) -> Result<FileEntry> {
    let file = File::open(&path)
        .with_context(|| format!("Failed to open {}", path.display()))?;

    let metadata = file.metadata()?;
    let size = metadata.len();

    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let file_count = stats.files.fetch_add(1, Ordering::Relaxed) + 1;
    let byte_count = stats.bytes.fetch_add(size, Ordering::Relaxed) + size;

    if file_count % 100 == 0 {
        println!(
            "Processed {:>8} files ({:.2} MB)",
            file_count,
            byte_count as f64 / 1_048_576.0
        );
    }

    Ok(FileEntry {
        path: path.to_string_lossy().to_string(),
        size,
        sha256: format!("{:x}", hasher.finalize()),
    })
}

fn write_json<T: Serialize>(filename: &str, data: &T) -> Result<()> {
    let file = File::create(filename)?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, data)?;
    writer.flush()?;
    Ok(())
}
