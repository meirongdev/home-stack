# 落点选 Cloudflare Workers 而非 Pages：Pages 跑不了常驻 Axum

> 日期: 2026-08-20
> 状态: ✅ 采纳
> 关联：[dual-target-axum](dual-target-axum.md)（这个决策让双目标编译成为必需）

## Context

选型时同时定下两件事：运行形态是「Axum + Maud + HTMX 常驻 SSR」，部署落点是
「Cloudflare Pages，和 blog 同款」（homelab blog 在 `meirongdevblog.pages.dev`）。

**这两条直接对撞。** Pages 只提供静态资源 + Functions（Workers 运行时），
**没有长驻进程** —— `axum::serve` 需要一个持续监听的 tokio runtime，Pages 上无处可放。
只能三选一：放弃 SSR、放弃 Cloudflare、或者换一个 Cloudflare 上的落点。

## Options

| 方案 | 结论 |
|---|---|
| **Cloudflare Workers + 静态资源** | ✅ 采纳 —— 同一账号/Terraform/DNS 模式，零集群开销，且能跑 Axum Router |
| Cloudflare Pages + 退回纯静态 SSG | ❌ 放弃了 HTMX 服务端交互（计算器 / Advisor 得整体挪到客户端 WASM） |
| oracle-k3s 常驻容器 | ❌ 见下：容量约束 + 六项运维负担 |
| Pages Functions 手写 JS | ❌ 站点核心是 Rust，为了迁就 Pages 而分裂成两种语言不成立 |

### 为什么不是 oracle-k3s

技术上完全可行（`personal-services` ns，走现成的 add-service 流程），但代价是六项：
一个 Pod、一条 PrometheusRule、一个 Uptime Kuma monitor、一个 Trivy 扫描目标、
一份 requests 预算、一次镜像构建流水线。

而 oracle 已从 4 OCPU/24GB 缩到 **2/12 且是单向操作**（homelab `docs/ROADMAP.md` 明确
「新服务别再按容量宽裕规划」）。一个只读、无状态、无数据库的目录站是最不该占这份容量的东西。

## Decision

**部署到 Cloudflare Workers（带静态资源），不用 Pages。**

支撑这个选择的三条事实：

1. **workers-rs 原生支持 Axum。** `worker` crate 的 `http` + `axum` feature 就是为此存在的 ——
   Axum 与 workers-rs 共用同一个 `http` crate，Router 可以被直接 `call()`。
   官方仓库 [`cloudflare/workers-rs/examples/axum`](https://github.com/cloudflare/workers-rs/tree/main/examples/axum)
   是一等公民示例，不是社区适配层。
2. **Workers 已有原生静态资源托管。** 2024-09 上线（取代旧的 Workers Sites），
   2026-03 起对静态资源、SSR、自定义域已与 Pages 完全对等。
3. **Cloudflare 官方建议新项目直接从 Workers 起步。** Pages 未废弃、存量项目照常运行，
   但后续投入都在 Workers 上。开新项目正好省掉「先上 Pages 再迁」这一步。

## Consequences

**得到**

- 零集群开销：0 pod / 0 PVC / 0 告警 / 0 monitor / 0 备份对象，见
  [../ARCHITECTURE.md](../ARCHITECTURE.md#对-homelab-的影响)。
- Terraform 改动只有一条 DNS 记录，加进 `cloudflare/terraform`。
- Workers 自定义域**走代理**（与 GitHub Pages 那两个主机名不同，不需要 DNS-only），
  因此仍在 WAF 后面；站点只读无状态，不需要动那条唯一的 rate limiting 规则
  （homelab `ROADMAP` #11：Free 计划只允许一条，已被 auth 端点 + Excalidraw relay 占用）。

**付出**

- 站点代码必须能编到 `wasm32-unknown-unknown`：没有 tokio net、native TLS、文件系统。
  这条约束的应对是把全部 I/O 关进构建期，见 [dual-target-axum](dual-target-axum.md)。
- Free 档上限：Worker 包体 ≤ 3 MiB gzip、CPU 10 ms/请求。当前规模远未触顶，
  触顶判据与出路见 [../ARCHITECTURE.md](../ARCHITECTURE.md#已知约束)。

**重评条件**（满足其一再议）

- 目录条目增长到 WASM 内嵌不再合适（估计几百条以上）→ 先改为 Workers KV / D1，Router 不动；
  仍不够再重评落点。
- 需要真正的运行时状态（用户账号、服务端持久化 shortlist）→ 那时 oracle-k3s + Postgres
  的六项代价才开始划算。
