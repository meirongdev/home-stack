//! `crates/edge` —— 线上入口（wasm32，Cloudflare Workers）。
//!
//! 这里只做三件事：装 panic hook、拿构建期内嵌的 `Catalog`、把请求交给
//! `site::router::app()`。**路由与模板一行都不在这里** ——
//! 那份 Router 由本 crate 与 `crates/dev` 共用，见
//! `docs/decisions/dual-target-axum.md`。
//!
//! 整个文件在非 wasm32 上被 cfg 掉：`worker` 编不到宿主平台，而
//! `cargo test --workspace` 在宿主上跑。宿主构建下这是一个空 crate。

#![cfg(target_arch = "wasm32")]

use tower_service::Service;
use worker::{event, Context, Env, HttpRequest, Result};

/// 内嵌内容构建出的 `Catalog`，每个 isolate 只解析一次。
///
/// wasm 是单线程，`OnceLock` 在这里没有竞争成本。
fn catalog() -> std::result::Result<&'static site::model::Catalog, String> {
    static CATALOG: std::sync::OnceLock<
        std::result::Result<site::model::Catalog, Vec<site::load::Issue>>,
    > = std::sync::OnceLock::new();
    match CATALOG.get_or_init(site::content::catalog) {
        Ok(c) => Ok(c),
        // **不 panic**：Worker 里 panic 就是一个没有信息的 500。
        // 内容在 CI 的 `xtask validate` 与 `render-diff` 已经过两道门禁，
        // 走到这里说明门禁被绕过了，那就把原因说清楚。
        Err(issues) => Err(format!(
            "内嵌内容未通过校验（{} 处）：{}",
            issues.len(),
            issues
                .iter()
                .map(|i| format!("{} @ {}:{}", i.message, i.path, i.line))
                .collect::<Vec<_>>()
                .join("; ")
        )),
    }
}

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    _env: Env,
    _ctx: Context,
) -> Result<axum::http::Response<axum::body::Body>> {
    console_error_panic_hook::set_once();

    let catalog = match catalog() {
        Ok(c) => c,
        Err(msg) => {
            return Ok(axum::http::Response::builder()
                .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                .header("content-type", "text/plain; charset=utf-8")
                .body(axum::body::Body::from(msg))
                .expect("构造错误响应"));
        }
    };

    Ok(site::router::app().with_state(catalog).call(req).await?)
}
