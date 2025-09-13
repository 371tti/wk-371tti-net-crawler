/// 先頭に https:// がなければ付与
fn add_https_if_missing(url: &str) -> String {
    let u = url.trim();
    if u.starts_with("http://") || u.starts_with("https://") {
        u.to_string()
    } else {
        format!("https://{}", u)
    }
}
use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use scraper::{Html, Selector, ElementRef};
use serde::Deserialize;
use serde_json::json;
use url::Url;
use headless_chrome::Browser;
use clap::ValueEnum;
use kurosabi::Kurosabi;
use tracing::{info, warn, error};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(author, version, about = "Open a URL and print the HTML using reqwest", long_about = None)]
struct Args {
    /// 取得するURL（JSON仕様モードでは不要）
    url: Option<String>,
    /// networkidle風に待つための静止時間(ms)。0で無効。
    #[arg(long, default_value_t = 0)]
    quiet_ms: u64,
    /// リクエスト全体のタイムアウト(ms)
    #[arg(long, default_value_t = 30_000)]
    timeout_ms: u64,
    /// 抽出するCSSセレクタ（指定しない場合はページ全体のHTMLを出力）
    #[arg(short, long)]
    selector: Option<String>,
    /// セレクタで一致した要素のinner HTMLを出力（デフォルトはテキスト）
    #[arg(long, conflicts_with = "attr")]
    html: bool,
    /// セレクタで一致した要素の属性を出力（例: --attr href）
    #[arg(long)]
    attr: Option<String>,
    /// 最初の一致のみ出力（デフォルトは全件）
    #[arg(long)]
    first: bool,
    /// JSON仕様ファイルのパス（指定時はURLやその他のCLI引数は無視）
    #[arg(long)]
    spec: Option<String>,
    /// JSON仕様を標準入力から受け取る
    #[arg(long)]
    spec_stdin: bool,
    /// 出力JSON（またはテキスト）を書き出すファイルパス。未指定時は標準出力
    #[arg(short, long)]
    out: Option<String>,
    /// 取得テキストの空白（連続空白/改行）を正規化して出力
    #[arg(long)]
    normalize: bool,
}

#[derive(Debug, Deserialize)]
struct JsonSpec {
    url: String,
    #[serde(default)]
    timeout_ms: Option<u64>,
    #[serde(default)]
    quiet_ms: Option<u64>,
    #[serde(default)]
    normalize: Option<bool>,
    #[serde(default)]
    render: Option<RenderOptions>,
    selectors: Vec<SelectorSpec>,
}

#[derive(Debug, Deserialize)]
struct SelectorSpec {
    name: String,
    selector: String,
    #[serde(default)]
    first: bool,
    #[serde(default)]
    unique: bool,
    #[serde(default)]
    output: Option<OutputSpec>,
    #[serde(default)]
    normalize: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct OutputSpec {
    #[serde(rename = "type")]
    kind: OutputKind,
    #[serde(default)]
    attr: Option<String>,
    #[serde(default)]
    absolute: bool,
    #[serde(default)]
    normalize: Option<bool>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct RenderOptions {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    wait: Option<WaitKind>,
    #[serde(default)]
    selector: Option<String>,
    #[serde(default)]
    timeout_ms: Option<u64>,
    /// DOMに変更が発生しない静止時間(ms)（wait=domidle時に使用）
    #[serde(default)]
    dom_idle_ms: Option<u64>,
}

#[derive(Debug, Deserialize, Clone, Copy, ValueEnum)]
#[serde(rename_all = "lowercase")]
#[value(rename_all = "lowercase")]
enum WaitKind { Load, Selector, Domcontentloaded, Domidle }

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum OutputKind { Text, Html, Attr }

fn main() {
    let mut app = kurosabi::Kurosabi::new();
    // ロガー初期化（RUST_LOG優先、なければinfo）
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .try_init();
    info!("server starting, registering routes");
    

    // POST /api -> JSON spec そのまま受け取り実行
    app.post("/api", |mut c| async move {
        info!("POST /api received");
        // 可能ならJSONとして直接パース、ダメならテキスト→JSON
        let spec_res: Result<JsonSpec, _> = c.req.body_de_struct::<JsonSpec>().await;
        let reply = match spec_res {
            Ok(mut spec) => {
                // url補正
                spec.url = add_https_if_missing(&spec.url);
                info!(url = %spec.url, "execute spec (POST)");
                match execute_spec(spec).await {
                Ok(out) => {
                    info!("POST /api success");
                    serde_json::to_string(&out).unwrap_or_else(|_| "{\"success\":false}".to_string())
                },
                Err(e) => {
                    error!(error = %e, "POST /api failed");
                    serde_json::to_string(&serde_json::json!({
                    "success": false,
                    "error": format!("{}", e)
                })).unwrap()
                }
            }
            },
            Err(_e) => {
                warn!("POST /api parse as struct failed, reading raw body");
                let body: String = c.req.body_string().await.unwrap_or_default();
                match serde_json::from_str::<JsonSpec>(&body) {
                    Ok(mut spec) => {
                        spec.url = add_https_if_missing(&spec.url);
                        info!(url = %spec.url, "execute spec (POST, raw body)");
                        match execute_spec(spec).await {
                        Ok(out) => {
                            info!("POST /api success");
                            serde_json::to_string(&out).unwrap_or_else(|_| "{\"success\":false}".to_string())
                        },
                        Err(e) => {
                            error!(error = %e, "POST /api failed");
                            serde_json::to_string(&serde_json::json!({
                            "success": false,
                            "error": format!("{}", e)
                        })).unwrap()
                        }
                    }
                    },
                    Err(e2) => {
                        warn!(error = %e2, "POST /api invalid json body");
                        serde_json::to_string(&serde_json::json!({
                        "success": false,
                        "error": format!("invalid json body: {}", e2)
                    })).unwrap()
                    }
                }
            }
        };
        c.res.json(&reply);
        c
    });

    // GET /url/* -> spec.sample.json を読み、url だけ差し替えて実行
    app.get("/url/*", |mut c| async move {
        // 完全なパスから /url/ の後ろ部分を抽出
        let full_path = &c.req.path.path;
        let url_part = if let Some(idx) = full_path.find("/url/") {
            &full_path[idx + 5..]
        } else {
            full_path
        };
        // クエリやフラグメントは消さずにそのまま
        let param = add_https_if_missing(url_part);
        info!(url = %param, "GET /url/* received");
        let spec_path = r"i:\RustBuilds\wk-371tti-net-crawler\spec.sample.json";
        let reply = match std::fs::read_to_string(spec_path)
            .ok()
            .and_then(|s| serde_json::from_str::<JsonSpec>(&s).ok())
        {
            Some(mut spec) => {
                spec.url = param;
                info!(url = %spec.url, "execute spec (GET)");
                match execute_spec(spec).await {
                    Ok(out) => {
                        info!("GET /url/* success");
                        serde_json::to_string(&out).unwrap_or_else(|_| "{\"success\":false}".to_string())
                    },
                    Err(e) => {
                        error!(error = %e, "GET /url/* failed");
                        serde_json::to_string(&serde_json::json!({
                        "success": false,
                        "error": format!("{}", e)
                    })).unwrap()
                    }
                }
            }
            None => {
                error!("failed to read or parse spec.sample.json");
                serde_json::to_string(&serde_json::json!({
                "success": false,
                "error": format!("failed to read or parse spec: {}", spec_path)
            })).unwrap()
            }
        };
        c.res.json(&reply);
        c
    });

    app.not_found_handler(|mut c| async move {
        c.res.text("404 Not Found");
        c.res.set_status(404);
        c
    });

    app.server().build().run();
}

fn has_class(el: &ElementRef, class: &str) -> bool {
    if let Some(c) = el.value().attr("class") {
        c.split_whitespace().any(|s| s == class)
    } else {
        false
    }
}


async fn execute_spec(spec: JsonSpec) -> Result<serde_json::Value> {
    // 既定値
    let timeout_ms = spec.timeout_ms.unwrap_or(30_000);
    let quiet_ms = spec.quiet_ms.unwrap_or(0);
    let normalize_global = spec.normalize.unwrap_or(false);
    info!(url = %spec.url, quiet_ms, timeout_ms, normalize = %normalize_global, "execute_spec start");

    // レンダリングが必要か分岐
    let render = spec.render.clone().unwrap_or_default();
    let (final_url, status, body) = if render.enabled {
            info!(wait = ?render.wait, selector = ?render.selector, timeout = ?render.timeout_ms, dom_idle_ms = ?render.dom_idle_ms, "render enabled");
            // Headless Chromeでナビゲートして待機
            let browser = Browser::default().context("failed to launch headless Chrome")?;
            let tab = browser.new_tab().context("failed to open new tab")?;
            tab.navigate_to(&spec.url)
                .with_context(|| format!("failed to navigate to {}", spec.url))?;

            // 待機条件
            match render.wait.unwrap_or(WaitKind::Load) {
                WaitKind::Load | WaitKind::Domcontentloaded => {
                    tab.wait_until_navigated().context("wait_until_navigated failed")?;
                }
                WaitKind::Selector => {
                    let sel = render.selector.as_deref().unwrap_or("body");
                    let to = Duration::from_millis(render.timeout_ms.unwrap_or(timeout_ms));
                    tab.wait_for_element_with_custom_timeout(sel, to)
                        .with_context(|| format!("selector not found within timeout: {}", sel))?;
                }
                                WaitKind::Domidle => {
                    let idle_ms = render.dom_idle_ms.unwrap_or(1000);
                    let max_wait = render.timeout_ms.unwrap_or(timeout_ms);
                    let start = std::time::Instant::now();
                    loop {
                                                let js = r#"(function(){
    if (!window.__wk_lastMutation) {
        window.__wk_lastMutation = Date.now();
        new MutationObserver(function(){ window.__wk_lastMutation = Date.now(); })
            .observe(document, {subtree:true, childList:true, attributes:true, characterData:true});
    }
    return Date.now() - window.__wk_lastMutation;
})()"#;
                                                let ro = tab.evaluate(js, false).context("failed to evaluate dom idle script")?;
                        let since_ms = ro.value.and_then(|v| v.as_f64()).unwrap_or(0.0);
                        if since_ms >= idle_ms as f64 { break; }
                        if start.elapsed() > Duration::from_millis(max_wait) { break; }
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
            }
                        if quiet_ms > 0 { std::thread::sleep(Duration::from_millis(quiet_ms)); }

                        // Remove script/style/noscript elements in the rendered DOM before
                        // extracting content so they don't pollute text extraction.
                        let remove_js = r#"(function(){
    document.querySelectorAll('script,style,noscript').forEach(function(e){ e.remove(); });
    return true;
})()"#;
                        let _ = tab.evaluate(remove_js, false).context("failed to remove script/style elements")?;
                        let html = tab.get_content().context("failed to get page content")?;
            let current_url = Url::parse(tab.get_url().as_str()).unwrap_or(Url::parse(&spec.url).unwrap());

            // ステータスは別途軽量に取得（必須でなければスキップ可）
            let client = reqwest::Client::builder()
                .user_agent("wk-371tti-net-crawler/0.1 (+https://example.invalid)")
                .redirect(reqwest::redirect::Policy::limited(10))
                .timeout(Duration::from_millis(timeout_ms))
                .brotli(true)
                .gzip(true)
                .deflate(true)
                .zstd(true)
                .build()
                .context("failed to build HTTP client")?;
            let status = client.get(current_url.as_str()).send().await.map(|r| r.status().as_u16()).unwrap_or(200);
            info!(status, url = %current_url, html_len = html.len(), "rendered and fetched status");
            (current_url, status, html)
        } else {
            // 通常のHTTP取得
            let client = reqwest::Client::builder()
                .user_agent("wk-371tti-net-crawler/0.1 (+https://example.invalid)")
                .redirect(reqwest::redirect::Policy::limited(10))
                .timeout(Duration::from_millis(timeout_ms))
                .brotli(true)
                .gzip(true)
                .deflate(true)
                .zstd(true)
                .build()
                .context("failed to build HTTP client")?;
            let resp = client
                .get(&spec.url)
                .send()
                .await
                .with_context(|| format!("failed to GET {}", spec.url))?;
            let status = resp.status().as_u16();
            let final_url = resp.url().clone();
            let body = resp.text().await.context("failed to read response body as text")?;
            if quiet_ms > 0 { tokio::time::sleep(Duration::from_millis(quiet_ms)).await; }
        info!(status, url = %final_url, body_len = body.len(), "http fetched");
            (final_url, status, body)
        };

    // 抽出
    let document = Html::parse_document(&body);
    // nameごとに配列へ集約
    let mut results: BTreeMap<String, Vec<String>> = BTreeMap::new();
    info!(selectors = spec.selectors.len(), "start selecting");
    for spec_item in spec.selectors.iter() {
        // ...従来通り...
        let selector = match Selector::parse(&spec_item.selector) {
            Ok(s) => s,
            Err(_e) => {
                warn!(selector = %spec_item.selector, "invalid CSS selector, skip");
                continue;
            }
        };
        let mut values: Vec<String> = Vec::new();
        // If the selector targets the whole page (body or html), don't require the
        // element to have .wk-visible — those root containers won't be leaf nodes
        // and thus won't receive the marker class in the render JS.
        let sel_trim = spec_item.selector.trim();
        let is_whole_page = sel_trim.eq_ignore_ascii_case("body") || sel_trim.eq_ignore_ascii_case("html");
        for el in document.select(&selector) {
            // skip <script> and <style> elements entirely
            if let Some(name) = el.value().name().to_lowercase().as_str().get(0..) {
                if name == "script" || name == "style" {
                    continue;
                }
            }
            // When rendered, prefer visible leaf nodes for descriptions unless the
            // selector explicitly requests the whole page container (body/html).
            if spec.render.as_ref().map(|r| r.enabled).unwrap_or(false)
                && spec_item.name == "descriptions"
                && !is_whole_page
            {
                if !has_class(&el, "wk-visible") { continue; }
            }
            let out = match &spec_item.output {
                Some(out) => match out.kind {
                    OutputKind::Text => normalize_if(el.text().collect::<String>(), spec_item.normalize, out.normalize, normalize_global),
                    OutputKind::Html => el.inner_html(),
                    OutputKind::Attr => {
                        if let Some(attr) = &out.attr {
                            if let Some(v) = el.value().attr(attr.as_str()) {
                                if out.absolute {
                                    normalize_if(absolutize_url(v, &final_url).unwrap_or_else(|| v.to_string()), spec_item.normalize, out.normalize, normalize_global)
                                } else {
                                    normalize_if(v.to_string(), spec_item.normalize, out.normalize, normalize_global)
                                }
                            } else { continue; }
                        } else { continue; }
                    }
                },
                None => normalize_if(el.text().collect::<String>(), spec_item.normalize, None, normalize_global),
            };
            if !out.is_empty() { values.push(out); }
            if spec_item.first { break; }
        }
        if spec_item.unique {
            values.sort();
            values.dedup();
        }
        results.entry(spec_item.name.clone()).or_default().extend(values);
    }

    // JSONへ変換
    let results_json: BTreeMap<String, serde_json::Value> = results
        .into_iter()
        .map(|(k, v)| (k, json!(v)))
        .collect();
    let total_items: usize = results_json.values().map(|v| v.as_array().map(|a| a.len()).unwrap_or(0)).sum();
    info!(keys = results_json.len(), total_items, "extraction done");

    let out_json = json!({
        "success": true,
        "url": final_url.as_str(),
        "status": status,
        "results": results_json,
    });
    Ok(out_json)
}

fn absolutize_url(input: &str, base: &Url) -> Option<String> {
    if let Ok(u) = Url::parse(input) {
        return Some(u.to_string());
    }
    base.join(input).ok().map(|u| u.to_string())
}

fn normalize_text(mut s: String) -> String {
    // 改行やタブをスペースに、複数空白を1つに、前後トリム
    // まずWindows系改行をLFへ
    s = s.replace(['\r', '\n', '\t'], " ");
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !prev_space { out.push(' '); prev_space = true; }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out.trim().to_string()
}

fn normalize_if(s: String, sel_norm: Option<bool>, out_norm: Option<bool>, global_norm: bool) -> String {
    let enabled = out_norm.or(sel_norm).unwrap_or(global_norm);
    if enabled { normalize_text(s) } else { s }
}
