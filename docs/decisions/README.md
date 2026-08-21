# Decisions

> 日期: 2026-08-20
> 关键技术决策记录（轻量 ADR）：当时的场景、可选项、为什么这么选。

| 决策 | 结论 |
|------|------|
| [cloudflare-workers-not-pages](cloudflare-workers-not-pages.md) | Pages 跑不了常驻 Axum → 落 Workers + 静态资源；否决 oracle-k3s（六项运维负担 + 缩容后无容量） |
| [dual-target-axum](dual-target-axum.md) | 一份 Router 两个编译目标（native 开发 / wasm32 线上）；`crates/site` 零 I/O 白送一个退回纯静态的逃生舱 |
| [typed-content-model-not-hugo](typed-content-model-not-hugo.md) | 分类法用 newtype 而非 String，四类静默事故变构建失败；⚠️ 别用「Rust 更快」为它辩护 |
| [field-notes-as-differentiator](field-notes-as-differentiator.md) | FieldNote 四态 Running/Retired/Rejected/Evaluating；每个数字必须能追回出处文档 |
| [calculator-advisor-justify-ssr](calculator-advisor-justify-ssr.md) | calculator / advisor 是差异化数据（footprint / status）的消费端，这正是押上 SSR 架构的正当化理由 |

> 本表按**依赖顺序**排，不按日期 —— 5 篇同日写成，日期给不出先后。
> 谁是谁的前提，看各篇文首的 `关联` 字段。

## 写新 ADR

- **命名**：描述性 kebab-case `<topic>.md`，日期写在文首、不靠文件名排序。
- **必含**：标题 / 日期 / 状态 / Context / Options / Decision / Consequences。
- **决策被推翻时不删旧文件**：把文首状态改成 `已废弃`，并链到取代它的记录。
