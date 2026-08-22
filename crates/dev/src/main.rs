//! `crates/dev` —— 本地开发入口（native）。
//!
//! tokio + axum::serve，`cargo run -p dev` → `:8080`。
//! 内容来自 `site::content`（构建期内嵌），**不在运行时读文件系统** ——
//! 和线上那一半用的是同一份数据，所以「本地看到的」与「线上渲染的」内容
//! 不可能不一致。改了 `content/` 下的 TOML，cargo 会因 `rerun-if-changed`
//! 重编 `crates/site`，再跑一次即生效。
//!
//! 唯一的例外是静态资源：Pagefind 索引由 `xtask build-site` 生成到 `public/`，
//! 线上由 Cloudflare 的资源层直接伺服，本地则由这里代劳（见 `serve_asset`）。

use site::model::Catalog;
use std::path::{Path, PathBuf};

fn main() {
    let catalog = match site::content::catalog() {
        Ok(c) => c,
        Err(issues) => {
            eprintln!("Catalog 校验未通过，无法启动：");
            for i in &issues {
                eprintln!("  error: {} （{}:{}）", i.message, i.path, i.line);
                if let Some(h) = &i.help {
                    eprintln!("    help: {h}");
                }
            }
            eprintln!("\n完整报错（带行列与源码行）跑：cargo run -p xtask -- validate");
            std::process::exit(1);
        }
    };
    let catalog: &'static Catalog = Box::leak(Box::new(catalog));

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("构建 tokio runtime 失败");
    rt.block_on(async move {
        // 资源层放在 Router 之前：线上是 Cloudflare 资源优先（命中即不唤起 Worker），
        // 本地要保持同样的顺序，否则两边对 /pagefind/* 的处理不一致。
        let app = axum::Router::new()
            .route("/pagefind/{*path}", axum::routing::get(serve_asset))
            .merge(site::router::app())
            .with_state(catalog);
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("绑定 :8080 失败");
        println!("dev server: http://{addr}");
        if !public_dir().join("pagefind").is_dir() {
            println!(
                "note: 还没有 dist/pagefind —— 站内搜索不可用。\
                 生成：cargo run -p xtask -- build-site"
            );
        }
        axum::serve(listener, app).await.expect("serve 失败");
    });
}

/// 资源层目录 —— 与 wrangler 上传的是同一个目录，
/// 这样「本地看到的资源」与「线上伺服的资源」不会是两套东西。
fn public_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../public")
}

/// 伺服 `public/pagefind/**` —— 只此一个前缀，且做了路径穿越防护。
///
/// 这段代码**只存在于 dev**：线上同样的路径由 Cloudflare 资源层应答，
/// Worker 压根不会被唤起（见 docs/ARCHITECTURE.md 的路由表）。
async fn serve_asset(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> axum::response::Response {
    use axum::http::{header, StatusCode};
    use axum::response::IntoResponse;

    // `..` 与绝对路径一律拒绝 —— 本地服务也不给自己开目录穿越。
    if path.split('/').any(|seg| seg == ".." || seg.is_empty()) {
        return (StatusCode::BAD_REQUEST, "bad path").into_response();
    }
    let full = public_dir().join("pagefind").join(&path);
    match std::fs::read(&full) {
        Ok(bytes) => {
            let ct = match full.extension().and_then(|e| e.to_str()) {
                Some("js") => "text/javascript; charset=utf-8",
                Some("css") => "text/css; charset=utf-8",
                Some("json") => "application/json",
                Some("wasm") => "application/wasm",
                _ => "application/octet-stream",
            };
            ([(header::CONTENT_TYPE, ct)], bytes).into_response()
        }
        Err(_) => (
            StatusCode::NOT_FOUND,
            "没有这个资源 —— 先跑 cargo run -p xtask -- build-site",
        )
            .into_response(),
    }
}
