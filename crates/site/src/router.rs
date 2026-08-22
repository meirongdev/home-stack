//! 唯一一份 axum Router —— 两个编译目标共用。
//!
//! 段 1 范围：SSR 页面路由（首页 / 分类 / vendor / 工具 / 404）。
//! 静态资源、HTMX 片段、结构化导出在段 2 补。

use crate::model::Catalog;
use crate::templates;
use axum::extract::{Path, State};
use axum::response::Html;
use axum::routing::get;
use axum::Router;

/// 构建 Router，状态为 `&'static Catalog`。
///
/// 调用方（`crates/dev` / `crates/edge`）负责 `.with_state(catalog)`。
pub fn app() -> Router<&'static Catalog> {
    Router::new()
        .route("/", get(home))
        .route("/domains/{domain}", get(domain_page))
        .route("/categories/{category}", get(category_page))
        .route("/replaces/{vendor}", get(vendor_page))
        .route("/tools/{slug}", get(tool_page))
        .fallback(not_found)
}

/// 全站可枚举的 URL —— **必须与上面的 `app()` 同步改动**。
///
/// 两个消费者：`xtask dump-html`（纯静态逃生舱 + Pagefind 索引的输入）与
/// `xtask render-diff`（两条内容路径逐字节比对）。
/// 一致性不靠自觉：`router_serves_every_enumerated_path` 拿这份清单逐条打进
/// `app()`，非 200 即测试失败。
///
/// 不含 `/pagefind/*` 这类静态资源 —— 那些不由 Router 应答。
pub fn all_paths(catalog: &Catalog) -> Vec<String> {
    let mut out = vec!["/".to_string()];
    for d in &catalog.domains {
        out.push(format!("/domains/{}", d.id));
    }
    for c in &catalog.categories {
        out.push(format!("/categories/{}", c.id));
    }
    for v in &catalog.vendors {
        // 零条目的 vendor 首页不索引，但页面本身仍然存在（会渲染成「暂无条目」）——
        // 静态导出要覆盖它，否则外部指进来的链接在纯静态形态下变 404。
        out.push(format!("/replaces/{}", v.id));
    }
    for t in &catalog.tools {
        out.push(format!("/tools/{}", t.slug));
    }
    out
}

/// 同步把一条路径打进 Router，返回 `(状态码, 响应体)`。
///
/// **两个消费者共用这一份**：本文件的路由一致性测试，与 `xtask dump-html` /
/// `render-diff`。导出的必须是「线上真正会返回的字节」——
/// 绕过 Router 直接调 `templates::*` 就是又一份渲染路径，必然漂移。
///
/// 不需要异步运行时：handler 是纯函数（`crates/site` 零 I/O 硬约束），
/// 一个 no-op waker 就够。`Pending` 直接 panic —— 那意味着有人往这里加了 I/O，
/// 而那在 wasm32 上会以更难看的方式失败。
pub fn render_path(catalog: &'static Catalog, path: &str) -> (axum::http::StatusCode, String) {
    use axum::body::Body;
    use axum::http::Request;
    use std::task::{Context, Poll, Waker};
    use tower_service::Service;

    fn poll_once<F: std::future::Future>(fut: F) -> F::Output {
        let mut fut = std::pin::pin!(fut);
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(v) => v,
            Poll::Pending => panic!("handler 返回了 Pending —— crates/site 不该有 I/O"),
        }
    }

    let mut app = app().with_state(catalog);
    let req = Request::builder()
        .uri(path)
        .body(Body::empty())
        .expect("构造请求");
    let res = poll_once(app.call(req)).expect("Router 不该出错");
    let status = res.status();
    let bytes = poll_once(axum::body::to_bytes(res.into_body(), usize::MAX)).expect("读响应体");
    (
        status,
        String::from_utf8(bytes.to_vec()).expect("响应体应是 UTF-8"),
    )
}

fn html(markup: maud::Markup) -> Html<String> {
    Html(markup.into_string())
}

async fn home(State(catalog): State<&'static Catalog>) -> Html<String> {
    html(templates::home(catalog))
}

async fn domain_page(
    State(catalog): State<&'static Catalog>,
    Path(domain): Path<String>,
) -> Html<String> {
    let id = crate::model::DomainId::new(domain);
    match templates::domain_page(catalog, &id) {
        Some(m) => html(m),
        None => html(templates::not_found()),
    }
}

async fn category_page(
    State(catalog): State<&'static Catalog>,
    Path(category): Path<String>,
) -> Html<String> {
    let id = crate::model::CategoryId::new(category);
    match templates::category_page(catalog, &id) {
        Some(m) => html(m),
        None => html(templates::not_found()),
    }
}

async fn vendor_page(
    State(catalog): State<&'static Catalog>,
    Path(vendor): Path<String>,
) -> Html<String> {
    let id = crate::model::VendorId::new(vendor);
    match templates::vendor_page(catalog, &id) {
        Some(m) => html(m),
        None => html(templates::not_found()),
    }
}

async fn tool_page(
    State(catalog): State<&'static Catalog>,
    Path(slug): Path<String>,
) -> Html<String> {
    let id = crate::model::Slug::new(slug);
    match templates::tool_page(catalog, &id) {
        Some(m) => html(m),
        None => html(templates::not_found()),
    }
}

async fn not_found() -> Html<String> {
    html(templates::not_found())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    /// `all_paths()` 与 `app()` 的一致性门禁：清单里每一条都必须真能被服务到。
    /// 加了新路由却忘了进清单 → 静态导出漏页；反之 → 导出一堆 404 页面。
    #[test]
    fn router_serves_every_enumerated_path() {
        let catalog = crate::content::catalog().expect("内嵌内容应当校验通过");
        let catalog: &'static Catalog = Box::leak(Box::new(catalog));
        let paths = all_paths(catalog);
        assert!(!paths.is_empty());
        for path in &paths {
            let (status, body) = render_path(catalog, path);
            assert_eq!(status, StatusCode::OK, "{path} 没有返回 200");
            assert!(!body.is_empty(), "{path} 返回了空响应体");
        }
    }

    /// 不存在的路径必须走到 fallback 的 404 页面 ——
    /// `xtask dump-html` 靠它生成 dist/404.html。
    #[test]
    fn unknown_path_renders_not_found_page() {
        let catalog = crate::content::catalog().expect("内嵌内容应当校验通过");
        let catalog: &'static Catalog = Box::leak(Box::new(catalog));
        let (status, body) = render_path(catalog, "/nope/nope");
        // fallback 渲染 404 页面，HTTP 状态码仍是 200（既有行为）——
        // 测试把它钉住，改动时必须是有意的。
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("没有这个页面"));
    }
}
