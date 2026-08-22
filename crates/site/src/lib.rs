//! `crates/site` —— 纯逻辑、零 I/O。
//!
//! 路由、模板、数据模型全部关在这里，两个入口（`crates/dev` native、
//! `crates/edge` wasm32）只做薄薄一层。零 I/O 是硬约束：每一行都必须能编到
//! `wasm32-unknown-unknown`，Markdown 渲染与数据抓取都被逼到构建期。
//!
//! 数据如何进来：`content::catalog()` 拿构建期内嵌的 TOML（由 `build.rs` 生成的表），
//! `load::load()` 则接收任意 `(path, raw)` 字符串对 —— 后者让 `xtask` 能从磁盘校验
//! 并给出精确行列号。**本 crate 的库代码自己不做任何文件读取**；
//! 唯一的 I/O 在构建脚本里，脚本不进产物。

pub mod content;
pub mod load;
pub mod model;
pub mod router;
pub mod templates;
