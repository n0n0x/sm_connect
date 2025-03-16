use anyhow::Result;
use home::home_dir;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};
use std::io::Write;
use std::time::SystemTime;
use std::{collections::HashMap, io::BufRead, path::PathBuf};

const RECENT_TIMEOUT: u64 = 60 * 60 * 24 * 30;

pub fn get_current_time() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("System time before Unix epoch")
        .as_secs()
}

pub struct History {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HistoryEntry {
    instance_id: String,
    when: u64,
}

impl HistoryEntry {
    pub fn new(instance_id: impl Into<String>) -> HistoryEntry {
        HistoryEntry {
            instance_id: instance_id.into(),
            when: get_current_time(),
        }
    }

    pub fn get_instance_id(&self) -> &str {
        &self.instance_id
    }

    pub fn get_when(&self) -> u64 {
        self.when
    }
}
// If we ever need to save the history and read it at the same time, we need to lock the file.
impl History {
    pub fn save(entry: HistoryEntry) -> Result<()> {
        let mut entries = Self::read()?;
        entries.insert(entry.get_instance_id().to_string(), entry);
        Self::write(&entries)
    }

    pub fn read() -> Result<HashMap<String, HistoryEntry>> {
        let mut entries = HashMap::new();
        let file_path = Self::get_history_path()?;
        
        // If file doesn't exist, return empty map
        if !file_path.exists() {
            return Ok(entries);
        }
        
        let file = std::fs::File::open(file_path)?;
        let current_time = get_current_time();
        let reader = std::io::BufReader::new(file);
        
        for line in reader.lines() {
            let line = line?;
            let entry: HistoryEntry = from_str(&line)?;
            
            // Skip expired entries
            if entry.when < current_time - RECENT_TIMEOUT {
                continue;
            }
            
            // Keep only the most recent entry for each instance
            entries
                .entry(entry.get_instance_id().to_string())
                .and_modify(|existing: &mut HistoryEntry| {
                    if existing.when < entry.when {
                        *existing = entry.clone();
                    }
                })
                .or_insert(entry);
        }
        
        Ok(entries)
    }

    fn write(entries: &HashMap<String, HistoryEntry>) -> Result<()> {
        let file = Self::get_history_path()?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(file)?;
        
        for entry in entries.values() {
            writeln!(file, "{}", to_string(entry)?)?;
        }
        Ok(())
    }

    pub fn reset() -> Result<()> {
        let file = Self::get_history_path()?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(file)?;
        file.flush()?;
        Ok(())
    }

    fn get_history_path() -> Result<PathBuf> {
        home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))
            .map(|home| home.join(".sm_connect_history"))
    }
}
