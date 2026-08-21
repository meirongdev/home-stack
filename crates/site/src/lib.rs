//! `crates/site` —— 纯逻辑、零 I/O。
//!
//! 路由、模板、数据模型全部关在这里，两个入口（`crates/dev` native、
//! `crates/edge` wasm32）只做薄薄一层。零 I/O 是硬约束：每一行都必须能编到
//! `wasm32-unknown-unknown`，Markdown 渲染与数据抓取都被逼到构建期。
//!
//! 数据如何进来：`Catalog::from_sources` 接收 `(path, raw)` 字符串对 ——
//! 本 crate 自己不做任何文件读取；读取由 `crates/dev` / `crates/xtask` 负责。

pub mod load;
pub mod model;
pub mod router;
pub mod templates;
