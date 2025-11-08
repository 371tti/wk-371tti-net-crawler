#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeResults {
    pub author: Vec<String>,
    pub base: Vec<String>,
    pub canonical: Vec<String>,
    pub content_html: Vec<String>,
    pub descriptions: Vec<String>,
    pub favicon: Vec<String>,
    pub headings: Vec<String>,
    pub lang: Vec<String>,
    pub links: Vec<String>,
    pub modified: Vec<String>,
    pub next: Vec<String>,
    pub prev: Vec<String>,
    pub published: Vec<String>,
    pub rss: Vec<String>,
    pub site_name: Vec<String>,
    pub tags: Vec<String>,
    pub title: Vec<String>,
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