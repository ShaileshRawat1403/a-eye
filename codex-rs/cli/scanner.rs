use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct SystemProfile {
    pub repo_type: String,
    pub frameworks: Vec<String>,
    pub languages: Vec<String>,
    pub package_manager: Option<String>,
    pub entrypoints: Vec<String>,
    pub verify_commands: Vec<String>,
    pub topology: String,
    pub risk_zones: Vec<String>,
    pub write_allowlist: Vec<String>,
    pub read_allowlist: Vec<String>,
}

pub struct Scanner;

impl Scanner {
    pub fn scan(repo_root: &Path) -> Result<SystemProfile> {
        let mut profile = SystemProfile::default();
        profile.topology = "single_service".to_string(); // Default for now

        let mut languages = HashSet::new();

        for entry in WalkDir::new(repo_root)
            .into_iter()
            .filter_entry(|e| !is_hidden(e))
            .filter_map(|e| e.ok())
        {
            let file_name = entry.file_name().to_string_lossy();

            if file_name == "package.json" {
                languages.insert("javascript".to_string());
                if profile.package_manager.is_none() {
                    profile.package_manager = Some("npm".to_string());
                }
                if repo_root.join("yarn.lock").exists() {
                    profile.package_manager = Some("yarn".to_string());
                }
                if profile.verify_commands.is_empty() {
                    profile.verify_commands.push("npm test".to_string());
                }
            } else if file_name == "Cargo.toml" {
                languages.insert("rust".to_string());
                profile.package_manager = Some("cargo".to_string());
                if profile.verify_commands.is_empty() {
                    profile.verify_commands.push("cargo test".to_string());
                }
            }
        }

        profile.languages = languages.into_iter().collect();
        profile.repo_type = if profile.languages.is_empty() { "unknown".to_string() } else { profile.languages.join(", ") };

        Ok(profile)
    }
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.') && s != ".")
        .unwrap_or(false)
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct SystemProfile {
    pub repo_type: String,
    pub frameworks: Vec<String>,
    pub languages: Vec<String>,
    pub package_manager: Option<String>,
    pub entrypoints: Vec<String>,
    pub verify_commands: Vec<String>,
    pub topology: String,
    pub risk_zones: Vec<String>,
    pub write_allowlist: Vec<String>,
    pub read_allowlist: Vec<String>,
}

pub struct Scanner;

impl Scanner {
    pub fn scan(repo_root: &Path) -> Result<SystemProfile> {
        let mut profile = SystemProfile::default();
        profile.topology = "single_service".to_string(); // Default for now

        let mut languages = HashSet::new();

        for entry in WalkDir::new(repo_root)
            .into_iter()
            .filter_entry(|e| !is_hidden(e))
            .filter_map(|e| e.ok())
        {
            let file_name = entry.file_name().to_string_lossy();

            if file_name == "package.json" {
                languages.insert("javascript".to_string());
                if profile.package_manager.is_none() {
                    profile.package_manager = Some("npm".to_string());
                }
                if repo_root.join("yarn.lock").exists() {
                    profile.package_manager = Some("yarn".to_string());
                }
                if profile.verify_commands.is_empty() {
                    profile.verify_commands.push("npm test".to_string());
                }
            } else if file_name == "Cargo.toml" {
                languages.insert("rust".to_string());
                profile.package_manager = Some("cargo".to_string());
                if profile.verify_commands.is_empty() {
                    profile.verify_commands.push("cargo test".to_string());
                }
            }
        }

        profile.languages = languages.into_iter().collect();
        profile.repo_type = if profile.languages.is_empty() { "unknown".to_string() } else { profile.languages.join(", ") };

        Ok(profile)
    }
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.') && s != ".")
        .unwrap_or(false)
}
}
