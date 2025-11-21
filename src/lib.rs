pub mod schema;

use std::time::Duration;

use reqwest::{Error, Response};
use urlencoding::encode;

use crate::schema::ScraperResult;



pub struct Client {
    pub base_url: String,
}

impl Client {
    /// Create a new Client with the specified base URL
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    /// Capture API
    /// screen capture API request builder
    pub fn capture_api(&self, url: &str) -> CaptureAPIBuilder {
        CaptureAPIBuilder::new(url)
    }

    /// Scraper API
    /// web scraping API request builder
    pub fn scraper_api(&self, url: &str) -> ScraperAPIBuilder {
        ScraperAPIBuilder::new(url)
    }
}

pub struct ScrapeAPI {
    pub url: String,
    pub selectors: Vec<String>,
    pub text_selector: Option<String>,
    pub waiting_selector: Option<String>,
}

pub struct CaptureAPI {
    pub url: String,
    pub selector: Option<String>,
    pub wait: Duration,
}

impl ScrapeAPI {
    pub(crate) fn generate_url(&self) -> String {
        let mut query = vec![format!("url={}", self.url)];
        for selector in &self.selectors {
            query.push(format!("selector={}", selector));
        }
        if let Some(text_sel) = &self.text_selector {
            query.push(format!("text_selector={}", text_sel));
        }
        if let Some(wait_sel) = &self.waiting_selector {
            query.push(format!("waiting_selector={}", wait_sel));
        }
        format!("/scraping?{}", query.join("&"))
    }

    /// Fetch the scraping result from the API
    pub async fn fetch(&self, client: &Client) -> Result<ScraperResult, Error> {
        let url = format!("{}{}", client.base_url, self.generate_url());
        let resp: Response = reqwest::get(&url).await?;
        let scraper_result: ScraperResult = resp.json().await?;
        Ok(scraper_result)
    }
}

impl CaptureAPI {
    pub(crate) fn generate_url(&self) -> String {
        let mut query = vec![format!("url={}", self.url)];
        if let Some(sel) = &self.selector {
            query.push(format!("selector={}", sel));
        }
        query.push(format!("wait={}", self.wait.as_millis()));
        format!("/capture?{}", query.join("&"))
    }

    /// Fetch the captured screenshot from the API
    pub async fn fetch(&self, client: &Client) -> Result<Vec<u8>, Error> {
        let url = format!("{}{}", client.base_url, self.generate_url());
        let resp: Response = reqwest::get(&url).await?.error_for_status()?;
        let bytes = resp.bytes().await?;
        Ok(bytes.to_vec())
    }
}

/// Builder for Scraper API requests
pub struct ScraperAPIBuilder {
    pub url: String,
    pub selectors: Vec<String>,
    pub text_selector: Option<String>,
    pub waiting_selector: Option<String>,
}

impl ScraperAPIBuilder {
    /// Create a new ScraperAPIBuilder with the specified URL
    pub fn new(url: &str) -> Self {
        ScraperAPIBuilder {
            url: encode(url).to_string(),
            selectors: Vec::new(),
            text_selector: None,
            waiting_selector: None,
        }
    }

    /// Add a selector to the list of selectors
    pub fn add_selector(mut self, selector: &str) -> Self {
        self.selectors.push(selector.to_string());
        self
    }

    /// Set the text selector
    pub fn set_text_selector(mut self, selector: &str) -> Self {
        self.text_selector = Some(selector.to_string());
        self
    }

    /// Set the waiting selector
    pub fn set_waiting_selector(mut self, selector: &str) -> Self {
        self.waiting_selector = Some(selector.to_string());
        self
    }

    /// Build the API request
    pub fn build(self) -> ScrapeAPI {
        ScrapeAPI {
            url: self.url,
            selectors: self.selectors,
            text_selector: self.text_selector,
            waiting_selector: self.waiting_selector,
        }
    }
}

/// Builder for Capture API requests
pub struct CaptureAPIBuilder {
    pub url: String,
    pub selector: Option<String>,
    pub wait: Duration,
}

impl CaptureAPIBuilder {
    /// Create a new CaptureAPIBuilder with the specified URL
    pub fn new(url: &str) -> Self {
        CaptureAPIBuilder {
            url: encode(url).to_string(),
            selector: None,
            wait: Duration::from_secs(0),
        }
    }

    /// Set the selector
    pub fn set_selector(mut self, selector: &str) -> Self {
        self.selector = Some(selector.to_string());
        self
    }

    /// Set the wait duration
    pub fn set_wait(mut self, wait: Duration) -> Self {
        self.wait = wait;
        self
    }

    /// Set the wait duration in seconds
    pub fn set_wait_secs(mut self, secs: u64) -> Self {
        self.wait = Duration::from_secs(secs);
        self
    }

    /// Set the wait duration in milliseconds
    pub fn set_wait_millis(mut self, millis: u64) -> Self {
        self.wait = Duration::from_millis(millis);
        self
    }

    /// Build the API request
    pub fn build(self) -> CaptureAPI {
        CaptureAPI {
            url: self.url,
            selector: self.selector,
            wait: self.wait,
        }
    }
}