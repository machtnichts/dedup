use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub checksum: String,
}

