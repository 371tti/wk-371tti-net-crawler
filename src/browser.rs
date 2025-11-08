use std::{collections::{HashMap, HashSet}, error::Error, sync::Arc};

use ego_tree::NodeRef;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use scraper::{Html, Node, Selector};

use crate::schema::ScrapeResults;

pub struct Engine {
    pub browser: Arc<Browser>,
}

impl Engine {
    
    const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36 (+https://371tti.net)";

    pub fn new(builder: LaunchOptionsBuilder) -> Result<Self, Box<dyn Error>> {
        let browser = Browser::new(builder.build().unwrap())?;
        Ok(Engine { browser: Arc::new(browser) })
    }

    pub fn capture_element(
        &self,
        url: &str,
        selector: &str,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let tab = self.browser.new_tab()?;
        tab.set_user_agent(Self::UA, None, None)?;
        let viewport = tab
            .navigate_to(url)?
            .wait_for_element(selector)?
            .get_box_model()?
            .margin_viewport();

        let png_data = tab.capture_screenshot(
            headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
            Some(100),
            Some(viewport),
            true,
        )?;

        Ok(png_data)
    }

    pub fn scraping(
        &self,
        url: &str,
        selector: Vec<&str>,
        text_selector: Option<&str>,
        waiting_selector: Option<&str>,
    ) -> Result<ScrapeResults, Box<dyn Error>> {
        let tab = self.browser.new_tab()?;
        tab.set_user_agent(Self::UA, None, None)?;
        tab.navigate_to(url)?.wait_for_element(
            waiting_selector.unwrap_or("html"),
        )?;
        let element = tab.wait_for_element("html")?;

        let url = tab.get_url();
        let base_url = url.split('/').take(3).collect::<Vec<&str>>().join("/");

        let document = element.get_content()?;
        tab.close(true)?;

        // parse ready
        let mut text = String::new();
        let fragments = Html::parse_document(&document);
        let text_selector = Selector::parse(text_selector.unwrap_or("html")).unwrap();
        let mut seen = HashSet::new();
        for el in fragments.select(&text_selector) {
            if seen.insert(el.id()) {
                let node = fragments.tree.get(el.id()).unwrap();
                Self::collect_plain_text(node, &mut text);
            }
        }

        // selectors
        let links_selector = Selector::parse("a[href]").unwrap();
        let favicon_selector = Selector::parse(r#"link[rel="icon"]"#).unwrap();
        let title_selector = Selector::parse("title").unwrap();
        let lang_selector = Selector::parse("html").unwrap();

        let links: Vec<String> = fragments.select(&links_selector)
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

        let contents: HashMap<String, String> = selector
            .iter()
            .map(|s| {
                let sel = Selector::parse(s).unwrap();
                let texts: Vec<String> = fragments.select(&sel)
                    .map(|elem| elem.text().collect::<String>().trim().to_string())
                    .collect();

                let joined_text = texts.join(" ");
                (s.to_string(), joined_text)
            })
            .collect();

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

    fn collect_plain_text(node: NodeRef<Node>, out: &mut String) {
        for child in node.children() {
            match child.value() {
                Node::Text(t) => {
                    let s = t.trim();
                    if !s.is_empty() {
                        if !out.is_empty() {
                            out.push(' ');
                        }
                        out.push_str(s);
                    }
                }
                Node::Element(e) => {
                    let tag = e.name();
                    if tag != "script" && tag != "style" {
                        Self::collect_plain_text(child, out);
                    }
                }
                _ => {}
            }
        }
    }
}

impl Default for Engine {
    fn default() -> Self {
        let mut builder = LaunchOptionsBuilder::default();
        builder
            .disable_default_args(true)
            .headless(true)
            .window_size(Some((2560, 1440)))
            .sandbox(true);
        Engine::new(builder).expect("Failed to create Engine")
    }
}
