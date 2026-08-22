//! 构建期内嵌的内容源（由 `build.rs` 生成的表）+ 一个已校验的 `Catalog` 入口。
//!
//! 这是**线上那一半唯一的内容来源** —— wasm32 上没有文件系统。
//! 两个编译目标都从这里拿数据，所以「dev 与线上内容不一致」在结构上不可能发生。

include!(concat!(env!("OUT_DIR"), "/content.rs"));

/// 从内嵌内容构建并校验 `Catalog`。
///
/// 校验失败返回 `Issue` 列表 —— 调用方（`crates/dev` / `crates/edge`）自己决定
/// 是打印后退出还是渲染一个错误页。**不要在这里 panic**：edge 里 panic 等于 500。
pub fn catalog() -> Result<crate::model::Catalog, Vec<crate::load::Issue>> {
    crate::load::load(SOURCES)
}
