# Probe Directory

自托管可观测性工具目录，按「你要替换掉哪个 SaaS」索引，用 Rust 实现。

**目前每条条目给的是可核对的二手事实**：许可证、实现语言、部署形态、运行依赖、
上手前要知道的坑，以及回到上游仓库与文档的链接 —— 29 条全部到位。
收录范围限定在 homelab（k3s、单人运维）用得上的工具，判据见 [docs/ROADMAP.md](docs/ROADMAP.md#明确不做)。

计划中的差异化是**一手运维证据**（FieldNote：实测资源足迹、踩过的坑、监控盲区、
以及「评估过但没上」的理由，每个数字都能追回一份出处文档）。⚠️ **段 3 才落库，
目前 0/29 条有 FieldNote** —— 所以站点页面上不写这句承诺，等有内容再写。

> 判断某件事是否已生效，看文档文首的 `状态` 字段 ——
> 不要按「文档在这儿所以东西跑着」来读。

## 现在到哪一步

| | |
|---|---|
| 设计 | ✅ 已定：5 条 ADR + 2 份规格 |
| 实现 | ✅ 段 1：骨架与内容模型（本地可跑）+ 段 4 的 GitHub 侧（`xtask fetch` → 上游活跃度）。下一步是段 2（上边缘 + CI 双门禁） |
| 部署 | ⬜ 未部署。落点已定为 Cloudflare Workers，域名待定 |

## 文档

**[docs/](docs/README.md) 是入口索引**，设计事实都在那里。四个入口：

| 想知道什么 | 读这个 |
|-----------|--------|
| 整体长什么样 | [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — 双目标编译、路由分工、栈选型、已知约束 |
| 为什么是这个方案 | [docs/decisions/](docs/decisions/README.md) — 轻量 ADR，5 条 |
| 具体怎么建模 / 怎么抓数据 | [docs/plans/](docs/plans/README.md) — 带日期的规格，写完即冻结 |
| 还剩什么没做 | [docs/ROADMAP.md](docs/ROADMAP.md) — 开放项 + 四段实施顺序 + 明确不做 |

## 形态

一份 `axum::Router`，两个编译目标：本地 `tokio` 驱动拿秒级反馈循环，线上编到
`wasm32-unknown-unknown` 跑在 Cloudflare Workers 上，逻辑只有一份。
⬜ **线上那一半尚未实施** —— `crates/edge` 还不存在，本地这一半已经在跑。

✅ 已生效的是内容侧：TOML 条目在构建期反序列化成强类型 `Catalog`，分类法字段用 newtype
而非 `String`，引用写错一个字母就是构建失败，而不是静默生成一个空页面。

数据抓取全部关在构建期，`crates/site` 零 I/O。这条硬约束同时满足 wasm32 的编译要求、
保护 Workers 的 CPU 预算，并白送一个「退回纯静态站」的逃生舱。
逐条论证见 [docs/decisions/](docs/decisions/README.md)。

## 开发

```sh
.githooks/install     # 启用提交/推送门禁；每次新 clone 都要跑一次
cargo run -p dev      # 本地浏览 → http://127.0.0.1:8080
cargo run -p xtask -- validate   # 内容四类校验（写错 vendor 会硬失败）
```

`pre-commit` 查密钥、文件卫生与文档规则（毫秒级）；`pre-push` 再加上 `cargo fmt` /
`clippy` / `test`、`cargo check --target wasm32-unknown-unknown`，以及
`xtask validate` 与 `xtask render-diff`（render-diff 属段 2、还没实现，钩子自动跳过）。
详见 [.githooks/README.md](.githooks/README.md)。

## 尚未确定

项目定名（`Probe Directory` / `observability_directory` / `obs.meirong.dev` 三个名字在流通）、
域名、LICENSE、以及 homelab 出处文档是否公开可达。
见 [docs/ROADMAP.md](docs/ROADMAP.md) 的开放项。
