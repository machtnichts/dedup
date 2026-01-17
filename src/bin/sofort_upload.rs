use serde::Deserialize;
use std::env;
use std::fs;

#[derive(Debug, Deserialize)]
struct FileEntry {
    path: String,
    size: u64,
    sha256: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1️⃣ JSON-Datei aus Argument lesen
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <json-file>", args[0]);
        std::process::exit(1);
    }
    let json_file = &args[1];

    let json_text = fs::read_to_string(json_file)?;
    let data: Vec<Vec<FileEntry>> = serde_json::from_str(&json_text)?;

    // 2️⃣ WebDAV-Base-URL deiner Nextcloud
    let base_url = "https://nrwv2yxngcbjcw6n.myfritz.net/remote.php/dav/files/trwa";

    for group in data {
        let mut has_valid_sofort_upload = false;

        for file in &group {
            if let Some(rest) = file.path.split("/var/lib/docker/volumes/nextcloud_aio_nextcloud_data/_data/trwa/files/SofortUpload/Camera/").nth(1) {
                let parts: Vec<&str> = rest.split('/').collect();
                if parts.len() >= 2 {
                    let year = parts[0];
                    let month = parts[1];
                    if year.len() == 4 && month.len() == 2 {
                        if let Some(filename) = parts.get(2) {
                            if filename.starts_with(year) && filename[4..6] == *month {
                                has_valid_sofort_upload = true;
                                break;
                            }
                        }
                    }
                }
            }
        }

        if has_valid_sofort_upload {
            for file in &group {
                // Skip Datei im validen SofortUpload
                let mut skip = false;
                if let Some(rest) = file.path.split("/var/lib/docker/volumes/nextcloud_aio_nextcloud_data/_data/trwa/files/SofortUpload/Camera/").nth(1) {
                    let parts: Vec<&str> = rest.split('/').collect();
                    if parts.len() >= 2 {
                        let year = parts[0];
                        let month = parts[1];
                        if let Some(filename) = parts.get(2) {
                            if filename.starts_with(year) && filename[4..6] == *month {
                                skip = true;
                            }
                        }
                    }
                }

                if !skip {
                    if let Some(pos) = file.path.find("/trwa/files/") {
                        let rel_path = &file.path[pos + "/trwa/files/".len()..];
                        let encoded_path = urlencoding::encode(rel_path);
                        let url = format!("{}/{}", base_url, encoded_path);

                        // Fertiger curl-Befehl mit Platzhaltern
                        println!(
                            "curl -u USER:APP_PASSWORD -X DELETE \"{}\"",
                            url
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

