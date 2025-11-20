# wk-371tti-net-crawler

Rust製 ウェブスクレイピングAPIサーバ

## 概要
- JSON仕様でページ抽出・CSSセレクタ指定・レンダリング（Chromium headless）対応
- HTTP API（kurosabiベース）でPOST/GETからスクレイピング・スクリーンショット取得
- Windows/クロスビルド対応（MSVC/GNU両方）
- 並列・非同期処理（tokio）

## 主な機能
- CSSセレクタによるテキスト・属性抽出
- headlessブラウザによるJSレンダリング・待機
- ページ全体/要素単位のスクリーンショット
- APIサーバとして複数リクエスト同時処理
- クエリパラメータで柔軟な指定

## ビルド 起動
```powershell
cargo build --release
cargo run --release
```

## APIエンドポイント
### 1. サーバ稼働確認
`GET /` → "Scraping server is running !!"

### 2. スクリーンショット取得
`GET /capture?url=<URL>&selector=<CSS>&wait=<ms>`
- url: 必須。対象ページURL
- selector: 任意。CSSセレクタ（指定時はその要素のみ）
- wait: 任意。ミリ秒待機
- レスポンス: PNG画像

### 3. スクレイピング
`GET /scraping?url=<URL>&selectors=<CSS1;CSS2;...>&text_selector=<CSS>&waiting_selector=<CSS>`
- url: 必須。対象ページURL
- selectors: 任意。抽出CSSセレクタ（`;`区切り）
- text_selector: 任意。ページ全体のテキスト抽出用CSS
- waiting_selector: 任意。レンダリング待機用CSS
- レスポンス: JSON（抽出結果、タイトル、リンク、favicon等）

#### レスポンス例
```json
{
	"status": 200,
	"url": "https://example.com",
	"results": {
		"title": "Example",
		"contents": { "h1": "見出し", "p": "本文..." },
		"links": ["https://..."],
		"favicon": "https://.../favicon.ico",
		"lang": "ja",
		"document": "<html>...</html>",
		"text": "ページ全体のテキスト..."
	}
}
```

