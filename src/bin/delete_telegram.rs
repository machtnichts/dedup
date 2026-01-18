use serde::Deserialize;
use std::env;
use std::fs;
use std::io::{self, Write};

const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RESET: &str = "\x1b[0m";

#[derive(Debug, Deserialize)]
struct FileEntry {
    path: String,
//    size: u64,
//    sha256: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ─────────────────────────────────────────────
    // Command-line arguments
    // ─────────────────────────────────────────────
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 || args.len() > 5 {
        eprintln!(
            "Usage: {} <json-file> <user> <app-password> [--dry-run | --no-dry-run]",
            args[0]
        );
        std::process::exit(1);
    }

    let json_file = &args[1];
    let user = &args[2];
    let password = &args[3];

    let mut dry_run = true;
    if args.len() == 5 {
        match args[4].as_str() {
            "--dry-run" => dry_run = true,
            "--no-dry-run" => dry_run = false,
            _ => {
                eprintln!("Unknown flag: {}", args[4]);
                std::process::exit(1);
            }
        }
    }

    if dry_run {
        println!("{YELLOW}⚠️  DRY-RUN mode enabled – no files will be deleted{RESET}");
    } else {
        println!("{RED}🔥 LIVE MODE – files will be permanently deleted!{RESET}");
    }

    // ─────────────────────────────────────────────
    // Safety confirmation
    // ─────────────────────────────────────────────
    if !dry_run {
        print!("{RED}Are you sure? Type 'DELETE' to continue:{RESET} ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input.trim() != "DELETE" {
            println!("Aborted.");
            return Ok(());
        }
    }

    // ─────────────────────────────────────────────
    // Load JSON
    // ─────────────────────────────────────────────
    let json_text = fs::read_to_string(json_file)?;
    let data: Vec<Vec<FileEntry>> = serde_json::from_str(&json_text)?;

    let base_url =
        "https://nrwv2yxngcbjcw6n.myfritz.net/remote.php/dav/files/trwa";
    let client = reqwest::blocking::Client::new();

    // ─────────────────────────────────────────────
    // Collect delete URLs according to policy
    // ─────────────────────────────────────────────
    let mut delete_urls = Vec::new();

    for group in &data {
        if let Some(indices_to_delete) = files_to_delete(group) {
            for idx in indices_to_delete {
                let file = &group[idx];

                if let Some(pos) = file.path.find("/trwa/files/") {
                    let rel_path = &file.path[pos + "/trwa/files/".len()..];

                    let encoded_path = rel_path
                        .split('/')
                        .map(|s| urlencoding::encode(s))
                        .collect::<Vec<_>>()
                        .join("/");

                    let url = format!("{}/{}", base_url, encoded_path);
                    delete_urls.push(url);
                }
            }
        } else {
            // Optionally log skipped groups
            // println!("Skipping group: no policy decision made");
        }
    }

    let total = delete_urls.len();
    println!("🗑️  Files scheduled for deletion: {}", total);

    // ─────────────────────────────────────────────
    // Delete with progress indicator
    // ─────────────────────────────────────────────
    let mut ok_count = 0;
    let mut error_count = 0;

    for (idx, url) in delete_urls.iter().enumerate() {
        let current = idx + 1;
        let percent = (current as f64 / total as f64) * 100.0;

        print!("\r[{percent:5.1}%] {current} of {total}");
        io::stdout().flush()?;

        if dry_run {
            println!("\nDRY-RUN: would delete {}", url);
            ok_count += 1;
            continue;
        }

        match client
            .delete(url)
            .basic_auth(user, Some(password))
            .send()
        {
            Ok(resp) if resp.status().is_success() => ok_count += 1,
            Ok(resp) => {
                error_count += 1;
                eprintln!(
                    "\n{RED}❌ HTTP {} for {}{RESET}",
                    resp.status(),
                    url
                );
            }
            Err(e) => {
                error_count += 1;
                eprintln!(
                    "\n{RED}❌ Request error for {}: {}{RESET}",
                    url, e
                );
            }
        }
    }

    // ─────────────────────────────────────────────
    // Summary
    // ─────────────────────────────────────────────
    println!("\n\n──────── Summary ────────");
    println!("{GREEN}✔ Successful:{RESET} {}", ok_count);
    println!("{RED}✖ Errors:{RESET} {}", error_count);
    println!("────────────────────────");

    if error_count > 0 {
        std::process::exit(2);
    }

    Ok(())
}

//
// ────────────────────────────────────────────────
// Deletion policy (all decision logic lives here)
// ────────────────────────────────────────────────
//

/// Decide which files in a group should be deleted.
///
/// Returns:
/// - `Some(indices)` → indices of files in `group` that should be deleted
/// - `None` → we have no idea for this group, skip it entirely
fn files_to_delete(group: &[FileEntry]) -> Option<Vec<usize>> {
    let mut has_preferred_entry = false;

    for file in group {
        if is_preferred_entry(file) {
            has_preferred_entry = true;
            break;
        }
    }

    if !has_preferred_entry {
        return None; // explicit: we don't know what to delete
    }

    Some(
        group
            .iter()
            .enumerate()
            .filter(|(_, file)| !is_preferred_entry(file))
            .map(|(idx, _)| idx)
            .collect(),
    )
}

/// Check whether a file is a valid SofortUpload/Camera file.
fn is_preferred_entry(file: &FileEntry) -> bool {
    let prefix =
        "/var/lib/docker/volumes/nextcloud_aio_nextcloud_data/_data/trwa/files/SofortUpload/Telegram/";

    let _rest = match file.path.strip_prefix(prefix) {
        Some(_r) => return false,
        None => return true,
    };
}

