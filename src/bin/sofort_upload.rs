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
    size: u64,
    sha256: String,
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

    // Dry-run is enabled by default
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
        println!(
            "{YELLOW}⚠️  DRY-RUN mode enabled – no files will be deleted{RESET}"
        );
    } else {
        println!(
            "{RED}🔥 LIVE MODE – files will be permanently deleted!{RESET}"
        );
    }

    // ─────────────────────────────────────────────
    // Safety confirmation for live mode
    // ─────────────────────────────────────────────
    if !dry_run {
        print!(
            "{RED}Are you sure? Type 'DELETE' to continue:{RESET} "
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if input.trim() != "DELETE" {
            println!("Aborted.");
            return Ok(());
        }
    }

    // ─────────────────────────────────────────────
    // Load JSON file
    // ─────────────────────────────────────────────
    let json_text = fs::read_to_string(json_file)?;
    let data: Vec<Vec<FileEntry>> = serde_json::from_str(&json_text)?;

    // Nextcloud WebDAV base URL
    let base_url =
        "https://nrwv2yxngcbjcw6n.myfritz.net/remote.php/dav/files/trwa";

    let client = reqwest::blocking::Client::new();

    // ─────────────────────────────────────────────
    // Collect delete URLs (needed for progress display)
    // ─────────────────────────────────────────────
    let mut delete_urls = Vec::new();

    for group in &data {
        let mut has_valid_sofort_upload = false;

        // Check if the group contains at least one valid SofortUpload file
        for file in group {
            if let Some(rest) = file.path.split(
                "/var/lib/docker/volumes/nextcloud_aio_nextcloud_data/_data/trwa/files/SofortUpload/Screenshots/",
            ).nth(1) {
                let parts: Vec<&str> = rest.split('/').collect();
                if parts.len() >= 3 {
                    let year = parts[0];
                    let month = parts[1];
                    let filename = parts[2];

                    if year.len() == 4
                        && month.len() == 2
                        && filename.starts_with(year)
                        && &filename[4..6] == month
                    {
                        has_valid_sofort_upload = true;
                        break;
                    }
                }
            }
        }

        // If a valid SofortUpload exists, delete all other files in the group
        if has_valid_sofort_upload {
            for file in group {
                let mut skip = false;

                if let Some(rest) = file.path.split(
                    "/var/lib/docker/volumes/nextcloud_aio_nextcloud_data/_data/trwa/files/SofortUpload/Screenshots/",
                ).nth(1) {
                    let parts: Vec<&str> = rest.split('/').collect();
                    if parts.len() >= 3 {
                        let year = parts[0];
                        let month = parts[1];
                        let filename = parts[2];

                        if filename.starts_with(year)
                            && &filename[4..6] == month
                        {
                            skip = true;
                        }
                    }
                }

                if !skip {
                    if let Some(pos) = file.path.find("/trwa/files/") {
                        
                       let rel_path = &file.path[pos + "/trwa/files/".len()..];

                       let encoded_path = 
                           rel_path.split('/')
                           .map(|segment| urlencoding::encode(segment))
                           .collect::<Vec<_>>()
                           .join("/");

                        let url = format!("{}/{}", base_url, encoded_path);

                        delete_urls.push(url);
                    }
                }
            }
        }
    }

    let total = delete_urls.len();
    println!("🗑️  Files scheduled for deletion: {}", total);

    // ─────────────────────────────────────────────
    // Delete files with progress indicator
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
            Ok(resp) if resp.status().is_success() => {
                ok_count += 1;
            }
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

