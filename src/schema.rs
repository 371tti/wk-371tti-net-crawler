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
#[serde(tag = "success")]
pub enum ScraperResult {
    #[serde(rename = "true")]
    Success {
        status: u16,
        url: String,
        results: ScrapeResults,
    },
    #[serde(rename = "false")]
    Failed {
        error: String,
    },
}