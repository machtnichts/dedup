use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
struct FileEntry {
    path: String,
    size: u64,
    sha256: String,
}

fn main() -> anyhow::Result<()> {
    // JSON-Datei einlesen
    let data = fs::read_to_string("duplicates.json")?;
    
    // JSON parsen
    let mut groups: Vec<Vec<FileEntry>> = serde_json::from_str(&data)?;
    
    // Gruppen nach Gesamtgröße sortieren (größere Gruppen zuerst)
    groups.sort_by(|a, b| {
        let size_a: u64 = a.iter().map(|f| f.size).sum();
        let size_b: u64 = b.iter().map(|f| f.size).sum();
        size_b.cmp(&size_a) // absteigend
    });
    
    // Ergebnis speichern
    let output = serde_json::to_string_pretty(&groups)?;
    fs::write("duplicates_sorted.json", output)?;
    
    println!("Sortierung abgeschlossen. Ergebnis: duplicates_sorted.json");
    Ok(())
}

