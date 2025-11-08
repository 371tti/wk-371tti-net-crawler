use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeResults {
    pub url: String,
    pub title: Option<String>,
    pub contents: HashMap<String, Vec<String>>,
    pub lang: Option<String>,
    pub favicon: Option<String>,
    pub links: Vec<String>,
    pub document: String,
    pub text: String,
}

/// success が bool の API レスポンスに対応 (例: {"success":true, ...} / {"success":false, "error":...})
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScraperResult {
    Success {
        success: bool, // 常に true を想定
        status: u16,
        url: String,
        results: ScrapeResults,
    },
    Failed {
        success: bool, // 常に false を想定
        error: String,
    },
}