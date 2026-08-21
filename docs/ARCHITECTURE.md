# Architecture — 一份 Axum Router，两个编译目标

> 日期: 2026-08-22
> 状态: 🟡 部分已完成 —— `crates/site` / `dev` / `xtask` 已完成（段 1）；
> `crates/edge`、`wrangler.jsonc`、Pagefind 与 HTMX 交互待实施（段 2）

单页总览。选型理由不在这里，在 [decisions/](decisions/README.md)；
具体建模与数据抓取规格在 [plans/](plans/README.md)。

## 一句话

站点是**服务端渲染的**（Maud 模板 + HTMX 局部替换），但**没有常驻服务器** ——
同一份 `axum::Router` 编译到 `wasm32-unknown-unknown`，由 Cloudflare Workers 运行时驱动。
本地开发时另一个入口用 tokio 驱动同一个 Router，拿秒级反馈循环。

## Crate 布局

```
crates/
├── site/     纯逻辑，零 I/O —— 每一行都必须能编到 wasm32
│   ├── router.rs      axum::Router<&'static Catalog>，两个目标共用的唯一一份路由
│   ├── templates.rs   maud，编译期检查的 HTML
│   ├── model.rs       serde 强类型 Catalog（构建期已校验完）
│   └── load.rs        TOML → Catalog + 引用完整性校验（入参是字符串，不碰文件系统）
├── dev/      native  —— tokio + axum::serve，cargo run → :8080
├── edge/     wasm32  —— worker + #[event(fetch)]，worker-build → wrangler deploy ⬜ 段 2
└── xtask/    native  —— 构建期工具：validate / fetch（render-diff 属段 2）
```

**`crates/site` 零 I/O 是硬约束，不是风格偏好。** 它同时兑现三件事：满足 wasm32
的编译约束、把渲染与数据抓取逼到构建期（运行时零 CPU 尖峰）、白送一个退回纯静态站的
逃生舱。逐条论证见 [decisions/dual-target-axum.md](decisions/dual-target-axum.md)。

```
                    crates/site  （纯逻辑，零 I/O）
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
   crates/dev  [native]              crates/edge  [wasm32]
   tokio + axum::serve               worker + #[event(fetch)]
   cargo run  →  :8080               worker-build → wrangler deploy
```

两个入口各约 10 行，差异被压到最薄。理由见
[decisions/dual-target-axum.md](decisions/dual-target-axum.md)。

## 请求路由：什么走静态资源，什么进 Worker

Workers 的默认顺序是**资源优先** —— 命中静态资源直接返回，压根不唤起 Worker，也就不计 CPU。
只有未命中才落到 Router。所以「SSR 的站」实际上大部分请求都是静态成本。

| 路径 | 由谁应答 | 说明 |
|------|---------|------|
| `/static/*`、字体、`htmx.min.js` | 资源层 | 不进 Worker，零 CPU |
| `/pagefind/*` | 资源层 | 构建期生成的搜索索引，纯客户端消费 |
| `/`、`/tools/{slug}`、`/replaces/{v}`、`/categories/{c}` | Worker · Maud SSR | 资源未命中 → 落到 Router |
| `/calculator/estimate`、`/advisor/rank` | Worker · HTMX 片段 | 返回 HTML 片段而非 JSON |
| `/api/tools.json`、`/index.xml`、`/tools.csv` | Worker | 机器可读的结构化导出 |

`wrangler.jsonc` 里两处关键配置：

- `"not_found_handling": "none"` —— 未命中**不要**由资源层兜底，交给 Worker 渲染 404。
- `"run_worker_first": ["/calculator/*", "/advisor/*", "/api/*"]` —— 这几条永远不该被同名资源遮蔽。

## 技术栈

已进依赖树的只有 `axum` / `maud` / `serde` / `toml` / `tokio`（+ axum 带进来的
`tower-service`），版本**以 `Cargo.lock` 的实际解析为准**，`cargo update` 之后回填本表。
`worker` / `worker-macros` / htmx / Pagefind 属段 2，`pulldown-cmark` 要等长文字段真的用上
—— 这四类目前都还没进 `Cargo.toml`，表里的版本是**选型结论，不是现状**。

| 组件 | 版本 | 为什么是它 |
|------|------|-----------|
| `axum` | 0.8.9 | Router 能被 workers-rs 直接 `call()`；⚠️ 0.8 起路径参数是 `{param}` 不是 `:param` |
| `worker` / `worker-macros` | 0.8.5 | Cloudflare 官方 SDK，`axum` feature 是一等支持而非社区适配层 |
| `maud` | 0.27.0 | 编译期检查的 HTML 宏。标签闭合、转义、类型全在 `cargo build` 抓 |
| `tower-service` | 0.3.3 | 提供 `Service::call`，Router 与 Workers 之间的唯一胶水 |
| `pulldown-cmark` | 0.13.4 | 长文 Markdown → HTML，**只在构建期跑** |
| `serde` | 1.0.229 | TOML 条目 → 强类型 `Catalog` |
| htmx | 2.0.10 | 计算器 / Advisor 的交互。约 14 KB，作静态资源发，零构建步骤 |
| Pagefind | 1.5.2 | 搜索。本身是 Rust 写的，构建期索引 `dist/`，运行时纯客户端，**不消耗 Worker CPU** |
| `tokio` | 1.53.1 | 仅 `crates/dev` 与 `crates/xtask`，**绝不进 edge 依赖树** |

⚠️ `crates/edge` 的 `axum` 必须 `default-features = false` —— 不关默认 feature 会拖进
tokio/hyper，wasm32 编不过（完整上下文见
[decisions/dual-target-axum.md](decisions/dual-target-axum.md)）。

## 已知约束

| 约束 | 实际影响 | 触顶时怎么办 |
|------|---------|-------------|
| Worker 包体 ≤ 3 MiB gzip（Free） | Rust WASM 基线约 300–600 KiB gz；29 条目录 TOML 内嵌约 33 KiB。到几百条都宽裕 | 目录移入 Workers KV 或 D1，Router 一行不用改 |
| CPU 10 ms / 请求（Free） | Maud 渲染是微秒级，不构成压力 | **前提是 Markdown 在构建期就渲成 HTML**，绝不在 handler 里 parse |
| 依赖必须编到 `wasm32-unknown-unknown` | 没有 tokio net、没有 native TLS、没有文件系统 | 全部 I/O 已关进 `crates/xtask`，这条约束不产生实际成本 |
| JS 支撑的 future 不是 `Send` | 只在 handler 内调 `worker::Fetch` 时才会咬人 | 运行时不发出站请求，天然规避 |
| `getrandom` 在 wasm32 需指定后端 | 一次性配置，几行 cfg | — |
| Rust → WASM 构建比 Hugo 慢 | 冷构建分钟级 | 日常开发只跑 `cargo run -p dev`（秒级），只有部署才编 wasm |

## 对 homelab 的影响

选 Cloudflare 而非 oracle-k3s 的理由（oracle 单向缩容至 2 OCPU/12GB + 六项运维负担）
不在这里复述，见
[decisions/cloudflare-workers-not-pages.md](decisions/cloudflare-workers-not-pages.md#为什么不是-oracle-k3s)。
结论是：这里的最优解是干脆不占容量。

| 这个站给 homelab 增加了什么 | |
|---|---|
| Pod / PVC / 那台 12 GB VM 上的内存 | 0 |
| PrometheusRule / Uptime Kuma monitor / Trivy 扫描目标 | 0 |
| restic 备份对象 | 0（内容即 git 仓库） |
| Terraform 改动 | 1 条 DNS 记录（`cloudflare/terraform`） |
| Homepage 磁贴 | 1 |

⚠️ 唯一的反向依赖是**构建期**从 homelab Prometheus 抓实测数据 —— 该步骤必须 fail-soft，
否则等于把公网站点的构建绑上了 homelab 的可用性。判据与实现见
[plans/2026-08-20-data-pipeline.md](plans/2026-08-20-data-pipeline.md)。

与 GitHub Pages 那两个主机名不同，**Workers 自定义域是走代理的** —— 不需要 DNS-only，
也就不绕过 WAF，且这个站只读无状态，不需要新增 rate limiting 规则
（homelab `ROADMAP` #11：Free 计划只允许一条，详情见
[decisions/cloudflare-workers-not-pages.md](decisions/cloudflare-workers-not-pages.md)）。
