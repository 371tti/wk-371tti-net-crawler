use std::error::Error;

pub mod browser;
pub mod schema;

fn browse_wikipedia() -> Result<(), Box<dyn Error>> {
    let engine = browser::Engine::default();

    let res = engine.scraping("https://ja.wikipedia.org/wiki/%E6%B7%B1%E6%B5%B7", vec!["h2"], Some("#bodyContent:not(.vector-body-before-content):not(.hatnote)"), None)?;

    for content in res.contents {
        println!("Content: {} / {}", content.0, content.1);
    }
    println!("URL: {}", res.url);
    println!("Language: {:?}", res.lang);
    println!("Favicon: {:?}", res.favicon);
    println!("Title: {:?}", res.title);
    println!("Links: {:?}", res.links);
    println!("Text: {}", res.text);

    Ok(())
}

fn main() {
    if let Err(err) = browse_wikipedia() {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}