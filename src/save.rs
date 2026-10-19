// src/save.rs — Persistent game saves

use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameSave {
    pub initial_fen: String,
    pub moves: Vec<String>,
    pub white_name: String,
    pub black_name: String,
    pub time_control: String,
    pub result: String,
    pub date: String,
    #[serde(default)]
    pub incomplete: bool,
}

fn saves_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let dir = PathBuf::from(home).join(".config").join("rchess").join("saves");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn auto_save_game(save: &GameSave) {
    let dir = saves_dir();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let status = if save.incomplete { "incomplete" } else { "finished" };
    let clean = |s: &str| -> String { s.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect() };
    let filename = format!("{}_{}_{}v{}.json", ts, status, clean(&save.white_name), clean(&save.black_name));
    let path = dir.join(filename);
    if let Ok(json) = serde_json::to_string_pretty(save) {
        let _ = fs::write(&path, json);
    }
}

pub fn list_saves() -> Vec<(PathBuf, GameSave)> {
    let dir = saves_dir();
    let mut results = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(save) = serde_json::from_str::<GameSave>(&content) {
                        results.push((path, save));
                    }
                }
            }
        }
    }
    results.sort_by(|a, b| b.1.date.cmp(&a.1.date));
    results
}
