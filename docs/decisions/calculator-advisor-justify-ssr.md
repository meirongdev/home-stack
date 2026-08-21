# 为什么为 calculator / advisor 押上 SSR：它是差异化数据的消费端

> 日期: 2026-08-20
> 状态: ✅ 采纳
> 关联：[field-notes-as-differentiator](field-notes-as-differentiator.md)（它提供数据，本决策消费数据）、
> [cloudflare-workers-not-pages](cloudflare-workers-not-pages.md) 与
> [dual-target-axum](dual-target-axum.md)（本决策是他们「选择复杂架构」的前提成立条件）

## Context

落点决策（cloudflare-workers-not-pages）和双目标编译（dual-target-axum）都只是一句话带过
一个前提——「站点是常驻 SSR，所以不能上 Pages」。但这两条决策真正压着的是
`crates/site` 那套复杂度的**全部载荷**：Rust + Axum + maud + wasm32 + 双入口。

而撑起「为什么不退成纯静态站」的，一直是这两个 HTMX 服务端交互：
`/calculator/estimate`（迁某 SaaS 到自托管省多少）和 `/advisor/rank`（按场景推荐工具）。
奇怪的是它们自己从未被论证过——文档把「这是选 Axum 的理由本身」当公理，
却不回答「它俩到底值多少，值不值整个架构压在上面」。
本文正式定义这两个交互的功能，并把这笔账算清，让这个隐假设显式化。

> 本 ADR 同时是对两个交互的功能定义：calculator = 迁移成本估算，
> advisor = 场景化工具推荐。此前它们在文档里只以路由路径出现，未定义行为。

## Options

| 方案 | 结论 |
|------|------|
| **保留服务端交互（HTMX），把它们定位为差异化数据的消费端** | ✅ 采纳 |
| 退纯静态 + 客户端 JS 实现同样功能 | ❌ 逻辑被迫在 JS 里重写一份，与强类型 `Catalog` / FieldNote 漂移 |
| 不做交互，纯目录静态站 | ❌ 丢掉了差异化主线的另一半，价值闭环没闭合 |

## Decision

**保留 `/calculator` 与 `/advisor` 两个 HTMX 服务端交互，并明确：它们不是独立 widget，
而是差异化数据（FieldNote / footprint）的消费端——所以「为它们押上 SSR」是正当的。**

### 为什么它们值得（核心论证）

1. **差异化是「数据 + 消费数据的交互」两条腿。**
   [field-notes-as-differentiator](field-notes-as-differentiator.md) 论证了 FieldNote
   为什么抄不走——实测足迹、Retired / Rejected 的理由，纯做目录的站永远拿不到，
   因为它没跑过任何东西。但数据要变成护城河，还得有人**用**它：
   - calculator 算出的「迁走 datadog 你省多少 / 本地要压多少内存」，系数来自 FieldNote
     的实测 footprint（Prometheus 抓的 234,532 series、2715Mi 峰值），不是拍脑袋的常量。
     字段能被抄，但「**从真实足迹算出省钱**」抄不走——那个输入数字，没跑过就是没有。
   - advisor 的排序吃 `status` / `replaces` / `categories` 这几个被类型校验过的字段，
     把「目录里平铺的 40 条」变成「按你的约束倒序给建议」；这个顺序只有数据真实存在
     才排得出来。
2. **为什么必须在服务端而非客户端。** 这两段逻辑吃的是同一份强类型 `Catalog`。
   放服务端 = 直接复用构建期已校验的数据与 Rust 类型，零重复实现；推到客户端 =
   用 JS 再实现一份「读 footprint / 排 status」的逻辑，必然和数据模型漂移——这和
   dual-target 的「一份逻辑两份实现必漂移」是同一种病，只是换个方向得。
   服务端渲染还保证「页面说推荐 X，目录里就真有 X」，不会出现不同步。
3. **为什么是 HTMX 片段而非 JSON / SPA。** 目录站的主体仍是静态页面（成本最低、
   SEO 干净、Pagefind 索引友好）。只在这两个点做服务端渲染的局部替换，不引前端框架、
   不把站点变成 SPA——复杂度被压在 `crates/site` 里，和静态页共用同一套 Maud 模板体系。

### 与架构决策的关系

这条 ADR 是 cloudflare-workers-not-pages 与 dual-target-axum 的**前提成立条件**：
没有这两个交互，选 Workers + 双目标编译就缺了主要回报。现在把前提写出来，
这两条决策才算自洽——「为了 X 选复杂架构」现在有了对 X 的明确定义。

## Consequences

**得到**

- 差异化价值闭环闭合：FieldNote（稀缺数据）→ calculator / advisor（消费数据、
  别人抄不走的交互）→ 回流（用户点进对应工具页 / FieldNote 出处）。
  字段能抄，但「从实测足迹算省钱」和「从实测状态排序」抄不走。
- 架构决策链条完整：四篇既有 ADR + 这篇，构成「为什么复杂架构是值得的」完整回答。
- 两个交互与 `crates/site` 共用一套强类型模型，天然和 Catalog 保持一致。

**付出**

- calculator 的估算模型本身需要设计：成本系数（SaaS 定价 vs 本地资源成本）从哪来、
  怎么定，是个需要维护的建模面——不能拍脑袋编，否则违反「数字要有出处」的纪律。
- advisor 的排序算法要定「约束 → 排序」的映射，同样是设计 + 维护面。
- 复杂度仍集中在段 1–2 的架构上；两个交互是「同一份 HTML」判据最容易暴露漂移的地方。

**重评条件**（满足其一再议）

- calculator / advisor 上线后使用率很低，或估算模型失真到误导读者——误导性内容
  比没有更糟（沿用 homelab RULES 的教训）。此时走逃生舱退纯静态站，删掉这两个
  handler 与 HTMX，架构复杂度随之下降。这条路是现成的——它是 dual-target 零 I/O
  约束的副产品，不需要额外设计，**风险有界**。
