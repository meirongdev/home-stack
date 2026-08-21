# 一份 Router 两个编译目标：wasm32 约束换来构建期纯净

> 日期: 2026-08-20
> 状态: ✅ 采纳
> 关联：[cloudflare-workers-not-pages](cloudflare-workers-not-pages.md)（本决策的前提）

## Context

落点定在 Cloudflare Workers 之后，站点代码必须编到 `wasm32-unknown-unknown`。
直接的后果是开发体验会很差：每次改一行模板都要走
`worker-build` → `wrangler dev` 这条分钟级的链路，而 Hugo 的反馈循环是毫秒级。

同时还有一个悬着的风险：**如果某个依赖编不到 wasm32 怎么办**。到那时项目已经全押在
Workers 上，退路成本很高。

## Options

| 方案 | 结论 |
|---|---|
| **共享 `crates/site` + 两个薄入口** | ✅ 采纳 —— 本地 tokio 拿秒级循环，线上 wasm32，逻辑只有一份 |
| 只做 wasm32 目标，靠 `wrangler dev` 开发 | ❌ 反馈循环分钟级；且没有退路 |
| 只做 native 目标 + SSG，放弃 SSR | ❌ 放弃 HTMX 服务端交互（这是选 Axum 的理由本身） |
| 两套代码（本地一套、边缘一套） | ❌ 必然漂移，等于没有本地开发 |

## Decision

把路由、模板、数据模型全部关进 **`crates/site`** —— 一个**零 I/O** 的 crate，
再由两个约 10 行的入口驱动：

- `crates/dev`（native）：`tokio` + `axum::serve` → `cargo run` → `:8080`
- `crates/edge`（wasm32）：`worker` + `#[event(fetch)]` → `worker-build` → `wrangler deploy`

同一份 Maud 模板、同一批类型、同一组 handler。布局见
[../ARCHITECTURE.md](../ARCHITECTURE.md#crate-布局)。

⚠️ `crates/edge` 的 `axum` 必须 `default-features = false` —— 不关默认 feature 会拖进
tokio/hyper，wasm32 编不过。这是整条路上最容易踩的一脚。

### 零 I/O 不是风格偏好

`crates/site` 不碰 I/O 这条约束同时兑现三件事，任何一件单独都不足以要求它，
三件叠起来才是这个设计的核心：

1. **满足编译约束** —— wasm32 上本来就没有 tokio net / native TLS / 文件系统。
2. **保护 CPU 预算** —— Markdown 渲染、数据抓取被逼到构建期，运行时不产生 CPU 尖峰，
   Free 档 10 ms/请求 因此有巨大余量。
3. **白送一个逃生舱** —— 见下。

## Consequences

**得到**

- 日常开发根本不碰 wasm 目标，`cargo run -p dev` 秒级。只有部署才编 wasm。
- **逃生舱不用额外设计**：既然 `crates/site` 零 I/O，`crates/dev` 加一个遍历路由
  dump HTML 的子命令就退回纯静态站 —— 照样能上 Cloudflare，也能塞进 k3s 的 nginx，
  只是失去 HTMX 那两个服务端交互。这是零 I/O 约束的副产品，不是额外工作。
- 因此段 1（骨架）与段 2（上边缘）之间有天然止损点，见 [../ROADMAP.md](../ROADMAP.md)。

**付出**

- 多两个 crate 的样板。实测代价是两个各约 10 行的文件。
- `crates/site` 里每加一个依赖都要问一句「它编得到 wasm32 吗」。
  这条纪律靠 CI 跑 `cargo check --target wasm32-unknown-unknown` 兜住，不靠人记。

**必须验证的事**（段 2 出口判据）

`crates/dev` 与线上必须渲染出**同一份 HTML**。两个目标共用逻辑是这个设计的全部价值，
一旦漂移，本地开发就失去意义 —— 所以它是判据，不是「顺便看看」。

这套判据要**机制化、可重复跑**，而不是实施时对比一次：给 `crates/xtask` 加一个
`render-diff` 子命令——对同一组参数遍历所有路由，用两个目标各自渲染出一份 HTML 做
逐字节 diff，不一致即硬失败（沿用 `validate` 的 hard-fail 语义）。接入 CI 后，
「两条入口不漂移」就从段 2 那一次的手工检查，变成每次合并都会跑的红线。
日常开发里 `render-diff` 也是本地快查：任何改动只要让两个目标渲染不一致就当场红，
在 dev 循环里就把漂移拦下，而不是等部署到线上才暴露。
