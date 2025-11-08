use std::{collections::HashMap, error::Error};
use std::sync::Arc;

use chromiumoxide::{Browser, BrowserConfig, Page, browser::HeadlessMode, cdp::browser_protocol::{emulation::{SetGeolocationOverrideParamsBuilder, SetTimezoneOverrideParamsBuilder}, page::{CaptureScreenshotFormat, ViewportBuilder}, target::CreateTargetParamsBuilder}, handler::viewport::Viewport, page::ScreenshotParamsBuilder};
use tokio::sync::Mutex as AsyncMutex;
use futures::StreamExt;
use scraper::{Html, Selector};

use crate::schema::ScrapeResults;

pub struct Engine {
    pub browser: Arc<AsyncMutex<Browser>>,
    pub handle: tokio::task::JoinHandle<()>,
}

impl Engine {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let (browser, mut handler) = Browser::launch(
            BrowserConfig::builder()
                .window_size(2560, 1440)
                .viewport(
                    Viewport {
                        width: 2560,
                        height: 1440,
                        device_scale_factor: None,
                        emulating_mobile: false,
                        is_landscape: true,
                        has_touch: false,
                    }
                )
                .disable_cache()
                .headless_mode(HeadlessMode::New)
                .build()?,
        ).await?;
        let browser = Arc::new(AsyncMutex::new(browser));
        let handle = tokio::task::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });
        Ok(Engine { browser, handle })
    }
    
    const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36 (+https://371tti.net)";

    pub async fn shutdown(&self) -> Result<(), Box<dyn Error>> {
        // Abort the background handler task (if still running) and close the browser.
        self.handle.abort();
        let mut b = self.browser.lock().await;
        let _ = b.kill().await;
        Ok(())
    }

    async fn new_page(&self, url: &str) -> Result<Page, Box<dyn Error>> {
        let b = self.browser.lock().await;
        b.clear_cookies().await?;
        let target_params = CreateTargetParamsBuilder::default()
            .url(url)
            .build()?;
        let page = b.new_page(target_params).await?;
        page.emulate_geolocation(
            SetGeolocationOverrideParamsBuilder::default()
                // 大阪日本橋 err 100m
                .latitude(34.6676)
                .longitude(135.5063)
                .accuracy(100)
                .build()
        ).await?;
        page.emulate_timezone(
            SetTimezoneOverrideParamsBuilder::default()
                .timezone_id("Asia/Tokyo")
                .build()?
        ).await?;
        page.enable_stealth_mode_with_agent(Self::UA).await?;
        Ok(page)
    }

    pub async fn capture_element(
        &self,
        url: &str,
        selector: &str,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let page = self.new_page(url).await?;

        page.wait_for_navigation().await?;

        let element = page.find_element(selector).await?;

        let bounding_box = element.bounding_box().await?;

        let viewport = ViewportBuilder::default()
            .x(bounding_box.x)
            .y(bounding_box.y)
            .width(bounding_box.width)
            .height(bounding_box.height)
            .scale(1.0)
            .build()?;

        let format = ScreenshotParamsBuilder::default()
            .format(CaptureScreenshotFormat::Png)
            .clip(viewport)
            .build();

        let png_data = page.screenshot(format).await?;

        page.close().await?;

        Ok(png_data)
    }

    pub async fn capture_full_page(
        &self,
        url: &str,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let page = self.new_page(url).await?;

        page.wait_for_navigation().await?;

        let format = ScreenshotParamsBuilder::default()
            .format(CaptureScreenshotFormat::Png)
            .full_page(true)
            .build();

        let png_data = page.screenshot(format).await?;

        page.close().await?;

        Ok(png_data)
    }

    pub async fn scraping(
        &self,
        url: &str,
        selector: Vec<&str>,
        text_selector: Option<&str>,
        waiting_selector: Option<&str>,
    ) -> Result<ScrapeResults, Box<dyn Error>> {
        let page = self.new_page(url).await?;

        page.wait_for_navigation().await?;

        page.find_element(waiting_selector.unwrap_or("html")).await?;

        let url = page.url().await?.ok_or("URL is None")?;
        let base_url = url.split('/').take(3).collect::<Vec<&str>>().join("/");

        let document = page.content().await?;
        let text_element = page.find_element(text_selector.unwrap_or("html")).await?;
        let text = text_element.inner_text().await?.unwrap_or(String::new());
        page.close().await?;

        // parse ready
        let fragments = Html::parse_document(&document);


        // selectors
        let links_selector = Selector::parse("a[href]").unwrap();
        let favicon_selector = Selector::parse(r#"link[rel="icon"]"#).unwrap();
        let title_selector = Selector::parse("title").unwrap();
        let lang_selector = Selector::parse("html").unwrap();

        let mut links: Vec<String> = fragments.select(&links_selector)
            .filter_map(|elem| elem.value().attr("href"))
            .map(|href| Self::url_normalize(&base_url, href))
            .collect();

        let favicon: Option<String> = fragments.select(&favicon_selector)
            .filter_map(|elem| elem.value().attr("href"))
            .map(|href| Self::url_normalize(&base_url, href))
            .next();

        let title: Option<String> = fragments.select(&title_selector)
            .filter_map(|elem| Some(elem.inner_html()))
            .next();

        let lang: Option<String> = fragments.select(&lang_selector)
            .filter_map(|elem| elem.value().attr("lang").map(|s| s.to_string()))
            .next();

        let contents: HashMap<String, Vec<String>> = selector
            .iter()
            .map(|s| {
                let sel = Selector::parse(s).unwrap();
                let texts: Vec<String> = fragments.select(&sel)
                    .map(|elem| elem.text().collect::<String>().trim().to_string())
                    .collect();

                (s.to_string(), texts)
            })
            .collect();

        links.sort();

        Ok(ScrapeResults {
            url,
            title,
            contents,
            lang,
            favicon,
            links,
            document,
            text,
        })
    }

    fn url_normalize(base_url: &str, href: &str) -> String {
        if href.starts_with("http://") || href.starts_with("https://") {
            href.to_string()
        } else {
            let base = base_url.trim_end_matches('/');
            if href.starts_with('/') {
                format!("{}/{}", base, href.trim_start_matches('/'))
            } else {
                format!("{}/{}", base, href)
            }
        }
    }
}