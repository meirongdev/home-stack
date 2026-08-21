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
        .route("/categories/{category}", get(category_page))
        .route("/replaces/{vendor}", get(vendor_page))
        .route("/tools/{slug}", get(tool_page))
        .fallback(not_found)
}

fn html(markup: maud::Markup) -> Html<String> {
    Html(markup.into_string())
}

async fn home(State(catalog): State<&'static Catalog>) -> Html<String> {
    html(templates::home(catalog))
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
