use std::error::Error;

use headless_chrome::LaunchOptionsBuilder;

pub struct Engine {
    pub browser: headless_chrome::Browser,
}

impl Engine {
    
    const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36 (+https://371tti.net)";

    pub fn new(builder: LaunchOptionsBuilder) -> Result<Self, Box<dyn Error>> {
        let browser = headless_chrome::Browser::new(builder.build().unwrap())?;
        Ok(Engine { browser })
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
        selector: &str,
    ) -> Result<String, Box<dyn Error>> {
        let tab = self.browser.new_tab()?;
        tab.set_user_agent(Self::UA, None, None)?;
        let element = tab.navigate_to(url)?.wait_for_element(selector)?;

        let content = element.inner_text()?;
        Ok(content)
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
