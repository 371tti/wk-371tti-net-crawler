#[cfg(not(feature = "lib"))]
use std::{borrow::Cow, sync::{Arc, Weak}, time::Duration};
#[cfg(not(feature = "lib"))]
use kurosabi::{Kurosabi, context::ContextMiddleware};
#[cfg(not(feature = "lib"))]
use urlencoding::decode;
#[cfg(not(feature = "lib"))]
use crate::{browser::Engine, schema::ScraperResult};
#[cfg(not(feature = "lib"))]
pub mod browser;
#[cfg(not(feature = "lib"))]
pub mod schema;
#[cfg(not(feature = "lib"))]
pub mod utils;
#[cfg(not(feature = "lib"))]
#[derive(Clone)]
pub struct ScraperContext {
    // store a Weak reference so the server's stored contexts do not keep the
    // Engine alive forever; handlers should attempt to upgrade when needed.
    pub engine: Weak<Engine>,
}
#[cfg(not(feature = "lib"))]
impl ScraperContext {
    /// Create a ScraperContext that holds a Weak reference to the engine.
    pub fn from_engine(engine: &Arc<Engine>) -> Self {
        ScraperContext { engine: Arc::downgrade(engine) }
    }
}
#[cfg(not(feature = "lib"))]
impl ContextMiddleware<ScraperContext> for ScraperContext {}
#[cfg(not(feature = "lib"))]
#[tokio::main(flavor = "multi_thread", worker_threads = 16)]
async fn main() {
    env_logger::try_init_from_env(env_logger::Env::default().default_filter_or("debug,selectors::matching=off,html5ever=off")).unwrap_or_else(|_| ());

    // Create the real Engine Arc and keep ownership in `engine_arc`.
    let engine = Engine::new().await.expect("Failed to initialize browser engine");
    let engine_arc = Arc::new(engine);
    // Create a context that holds only a Weak reference; this prevents the
    // server from keeping the Engine alive by accident.
    let ctx = ScraperContext::from_engine(&engine_arc);
    let mut kurosabi = Kurosabi::with_context(ctx.clone());

    kurosabi.get("/", |mut c| async move {
        c.res.text("Scraping server is running !!");
        c
    });

    // Capture screenshot endpoint
    // URL Query Parameters:
    // - url: URL to capture
    // - selector: (optional) CSS selector to capture only a specific element
    //
    kurosabi.get("/capture", |mut c| async move {
        let url = c.req.path.get_query("url");
        let wait_duration = c.req.path.get_query("wait")
            .and_then(|s| s.parse::<u64>().ok())
            .map(std::time::Duration::from_millis)
            .unwrap_or(std::time::Duration::from_millis(0));
        if let Some(url) = url {
            let url = decode(&url).unwrap_or_else(|_| Cow::Borrowed(url.as_str())).to_string();
            let selector = c.req.path.get_query("selector");
            if let Some(selector) = selector {
                // attempt to upgrade Weak -> Arc
                if let Some(engine) = c.c.engine.upgrade() {
                    let png_data = engine.capture_element(&url, &selector, wait_duration).await;
                    match png_data {
                        Ok(data) => {
                            c.res.binary(&data);
                            c.res.header.set("Content-type", "image/png");
                        }
                        Err(e) => {
                            c.res.text(&format!("Error capturing screenshot: {}", e));
                            c.res.set_status(500);
                        }
                    }
                } else {
                    c.res.text("Engine not available");
                    c.res.set_status(503);
                }
            } else {
                if let Some(engine) = c.c.engine.upgrade() {
                    let png_data = engine.capture_full_page(&url, wait_duration).await;
                    match png_data {
                        Ok(data) => {
                            c.res.binary(&data);
                            c.res.header.set("Content-type", "image/png");
                        }
                        Err(e) => {
                            c.res.text(&format!("Error capturing screenshot: {}", e));
                            c.res.set_status(500);
                        }
                    }
                } else {
                    c.res.text("Engine not available");
                    c.res.set_status(503);
                }
            }
        } else {
            c.res.text("Missing 'url' query parameter");
            c.res.set_status(400);
        }
        c
    });

    // Scraping endpoint
    // スクレイピング用のエンドポイント
    // Url Query Parameters:
    // - url: URL to scrape
    // - selectors: Semicolon-separated list of CSS selectors to extract contents. `;` is used as separator
    // - text_selector: (optional) CSS selector to extract text content
    // - waiting_selector: (optional) CSS selector to wait for before scraping
    //
    // Example:
    // /scraping?url=https://example.com
    // /scraping?url=https://ja.wikipedia.org/wiki/%E5%9C%8F%E8%AB%96&text_selector=.mw-body-content
    // 
    kurosabi.get("/scraping", |mut c| async move {
        let url = c.req.path.get_query("url");
        let selectors_owner = c.req.path.get_query("selectors")
            .map(|s| s.split(';').map(|item| item.to_string()).collect::<Vec<String>>())
            .unwrap_or_else(|| vec![]);
        let selectors = selectors_owner.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
        let text_selector = c.req.path.get_query("text_selector");
        let waiting_selector = c.req.path.get_query("waiting_selector");
        if let Some(url) = url {
            let url = decode(&url).unwrap_or_else(|_| Cow::Borrowed(url.as_str())).to_string();
            if let Some(engine) = c.c.engine.upgrade() {
                let result = engine.scraping(&url, selectors, text_selector.as_deref(), waiting_selector.as_deref()).await;
                match result {
                    Ok(scrape_results) => {
                        let result = ScraperResult::Success {
                            status: 200,
                            url: url.clone(),
                            results: scrape_results,
                        };
                        c.res.json_value(&serde_json::to_value(result).unwrap());
                    }
                    Err(e) => {
                        let result = ScraperResult::Failed {
                            error: format!("Error during scraping: {}", e),
                        };
                        c.res.json_value(&serde_json::to_value(result).unwrap());
                    }
                }
            } else {
                let result = ScraperResult::Failed {
                    error: "Engine not available".to_string(),
                };
                c.res.json_value(&serde_json::to_value(result).unwrap());
            }
        } else {
            let result = ScraperResult::Failed {
                error: "Missing 'url' query parameter".to_string(),
            };
            c.res.json_value(&serde_json::to_value(result).unwrap());
        }
        c
    });

    kurosabi.not_found_handler(|mut c| async move {
        c.res.text("invalid endpoint");
        c
    });

    // サーバをメインタスクで起動し、終了時にエンジンもshutdown
    let server = kurosabi.server()
        .host([0,0,0,0])
        .thread(16)
        .port(3773)
        .nodelay(true)
        .http_keepalive_timeout(Duration::from_secs(300))
        .build();

    println!("server started. Press Ctrl-C to shutdown...");

    tokio::select! {
        _ = server.run_async() => {
            println!("server stopped (run_async returned)");
        }
        _ = tokio::signal::ctrl_c() => {
            println!("received Ctrl-C, shutting down server and browser engine...");
        }
    }

    // サーバ停止後にエンジンもshutdown
    if let Err(e) = engine_arc.shutdown().await {
        eprintln!("engine shutdown error: {}", e);
    }
    println!("shutdown complete. Exiting.");
}