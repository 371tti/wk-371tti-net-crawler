use std::error::Error;

use headless_chrome::Browser;
use headless_chrome::protocol::cdp::Page;

fn browse_wikipedia() -> Result<(), Box<dyn Error>> {
    let browser = Browser::default()?;
    println!("{:?}", browser.get_version()?);

    let tab = browser.new_tab()?;

    // Navigate to wikipedia
    tab.navigate_to("https://www.wikipedia.org")?;

    // Wait for network/javascript/dom to make the search-box available
    // and click it.
    tab.wait_for_element("input#searchInput")?.click()?;

    // Type in a query and press `Enter`
    tab.type_str("WebKit")?.press_key("Enter")?;

    // We should end up on the WebKit-page once navigated
    let elem = tab.wait_for_element("#firstHeading")?;
    assert!(tab.get_url().ends_with("WebKit"));

    let jpeg_data = tab.capture_screenshot(
        Page::CaptureScreenshotFormatOption::Jpeg,
        None,
        None,
        true)?;
    // Save the screenshot to disc
    std::fs::write("screenshot.jpeg", jpeg_data)?;

    let png_data = tab
        .wait_for_element("#mw-content-text > div > table.infobox.vevent")?
        .capture_screenshot(Page::CaptureScreenshotFormatOption::Png)?;
    // Save the screenshot to disc
    std::fs::write("screenshot.png", png_data)?;

    // Run JavaScript in the page
    let remote_object = elem.call_js_fn(r#"
        function getIdTwice () {
            // `this` is always the element that you called `call_js_fn` on
            const id = this.id;
            return id + id;
        }
    "#, vec![], false)?;
    match remote_object.value {
        Some(returned_string) => {
            dbg!(&returned_string);
            assert_eq!(returned_string, "firstHeadingfirstHeading".to_string());
        }
        _ => unreachable!()
    };

    Ok(())
}


fn main() {
    if let Err(err) = browse_wikipedia() {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}