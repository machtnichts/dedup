use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;
use std::env;
use std::path::Path;
use dedup::types::FileEntry;

fn main() -> anyhow::Result<()> {
    let input_path = env::args()
        .nth(1)
        .expect("Usage: resort_all <input_file.json>");

    let file = File::open(&input_path)?;
    let reader = BufReader::with_capacity(1024*1024, file); //1mb buffer
    let mut groups: Vec<FileEntry> = serde_json::from_reader(reader)?;

    groups.sort_by(|a, b| {
        b.size.cmp( &a.size)
    });
    
    // store back
    let output_path = make_output_path(&input_path);
    let out = File::create(&output_path)?;
    let writer = BufWriter::with_capacity(1024*1024, out);
    serde_json::to_writer_pretty(writer, &groups)?;
    
    println!("done, {} written.", output_path);
    Ok(())
}


fn make_output_path(input: &str) -> String {
    let path = Path::new(input);

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("json");

    let parent = path.parent().unwrap_or_else(|| Path::new(""));

    parent
        .join(format!("{}_sorted.{}", stem, ext))
        .to_string_lossy()
        .into_owned()
}

