use std::error::Error;

use headless_chrome::{Browser, LaunchOptionsBuilder};
use headless_chrome::protocol::cdp::Page;

fn browse_wikipedia() -> Result<(), Box<dyn Error>> {
    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .disable_default_args(true)
            .headless(false)
            .build()
            .unwrap(),
    )?;
    println!("{:?}", browser.get_version()?);

    let tab = browser.new_tab()?;

    // Navigate to wikipedia
    tab.navigate_to("https://www.google.com/")?;

    // Wait for network/javascript/dom to make the search-box available
    // and click it.
    tab.wait_for_element("textarea")?.click()?;

    // Type in a query and press `Enter`
    tab.type_str("371tti")?.press_key("Enter")?;

    let jpeg_data = tab.capture_screenshot(
        Page::CaptureScreenshotFormatOption::Jpeg,
        None,
        None,
        true)?;
    // Save the screenshot to disc
    std::fs::write("screenshot.jpeg", jpeg_data)?;


    Ok(())
}


fn main() {
    if let Err(err) = browse_wikipedia() {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}