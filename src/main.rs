use std::error::Error;

pub mod browser;
pub mod schema;

use headless_chrome::{Browser, LaunchOptionsBuilder};
use headless_chrome::protocol::cdp::Page;

fn browse_wikipedia() -> Result<(), Box<dyn Error>> {
    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .disable_default_args(true)
            .headless(true)
            .window_size(Some((2560, 1440)))
            .sandbox(true)
            .build()
            .unwrap(),
    )?;
    println!("{:?}", browser.get_version()?);

    let tab = browser.new_tab()?;

    // Navigate to wikipedia
    let viewport = tab.navigate_to("https://wikipedia.org")?.wait_for_element("html")?.get_box_model()?.margin_viewport();

    let jpeg_data = tab.capture_screenshot(
        Page::CaptureScreenshotFormatOption::Png,
        Some(100),
        Some(viewport),
        true)?;
    // Save the screenshot to disc
    std::fs::write("screenshot.png", jpeg_data)?;
    println!("Screenshot saved to screenshot.png");


    Ok(())
}

fn main() {
    if let Err(err) = browse_wikipedia() {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}