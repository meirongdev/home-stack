# Home Stack

homelab 自托管技术选型目录，按**主题域**索引，用 Rust 实现。

**目前每条条目给的是可核对的二手事实**：许可证、实现语言、部署形态、运行依赖、
上手前要知道的坑，以及回到上游仓库与文档的链接 —— 97 条全部到位，
覆盖 10 个域（计算底座 / 网络 / 存储 / 交付 / 密钥 / 身份 / 可观测 / 安全 / IaC / 数据）。
收录范围限定在 homelab（k3s、单人运维）用得上的工具，判据见 [docs/ROADMAP.md](docs/ROADMAP.md#明确不做)。

计划中的差异化是**一手运维证据**（FieldNote：实测资源足迹、踩过的坑、监控盲区、
以及「评估过但没上」的理由，每个数字都能追回一份出处文档）。⚠️ **段 3 才落库，
目前 0/97 条有 FieldNote** —— 所以站点页面上不写这句承诺，等有内容再写。

> 判断某件事是否已生效，看文档文首的 `状态` 字段 ——
> 不要按「文档在这儿所以东西跑着」来读。

## 现在到哪一步

| | |
|---|---|
| 设计 | ✅ 已定：6 条 ADR + 2 份规格 + 2 份 reference（内容模型、[跨仓库边界](docs/reference/cross-repo-boundary.md)）|
| 实现 | ✅ 段 1 全部、段 2 的**代码**全部（wasm32 入口、构建期内容内嵌、静态导出、Pagefind 搜索、CI 九道门禁）、段 4 的 GitHub 侧（含夜间刷新）。内容覆盖 10 个域 / 97 条 |
| 部署 | ✅ **已上线**（2026-08-23）：<https://stack.meirong.dev> —— 4 个 Terraform 资源，实测包体 346 KiB gz（Free 上限 11%）。部署逻辑**可被别的项目复用**：`cloudflare/terraform/modules/worker` 是无 provider 的子模块，本仓库只是它的第一个消费者。✅ **部署已交给 CI**（2026-08-23）：state 在 R2（`terraform-backend/home-stack/`），`deploy.yml` 手动触发、在干净 runner 上首跑通过 —— 工作站不再是唯一能部署的机器 |
| 下一步 | 段 3（FieldNote）；以及 render-diff 的 wasm 运行时那一半（[ROADMAP](docs/ROADMAP.md) 开放项 13）|

## 目录的两根轴

**域**是第一根轴：读者先问「我在选什么」（CNI？密钥后端？备份方案？），
再在域内按分类收窄。**「替换哪个 SaaS」是第二根轴** —— 它仍然有效，
但大半基建条目（CNI、hypervisor、GitOps 控制面）压根没有对应的 SaaS，
所以它不能当第一根轴用。逐条论证见
[docs/decisions/domain-layer-not-flat-categories.md](docs/decisions/domain-layer-not-flat-categories.md)。

跨域的工具只登记**主域**（Cilium 记在网络，尽管它也做策略与流量可观测）——
换来的是「域内条目数」可加、面包屑有唯一父级。代价写在那份 ADR 的 Consequences 里。

## 文档

**[docs/](docs/README.md) 是入口索引**，设计事实都在那里。入口：

| 想知道什么 | 读这个 |
|-----------|--------|
| 整体长什么样 | [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — 双目标编译、路由分工、栈选型、已知约束 |
| 为什么是这个方案 | [docs/decisions/](docs/decisions/README.md) — 轻量 ADR，6 条 |
| 具体怎么建模 / 怎么抓数据 | [docs/plans/](docs/plans/README.md) — 带日期的规格，写完即冻结 |
| 还剩什么没做 | [docs/ROADMAP.md](docs/ROADMAP.md) — 开放项 + 四段实施顺序 + 明确不做 |
| 内容模型现在长什么样 | [docs/reference/content-model.md](docs/reference/content-model.md) — 字段、不变量、五类校验、内容如何进到两个目标里 |
| 怎么把它部署上去 | [docs/runbooks/deploy-cloudflare.md](docs/runbooks/deploy-cloudflare.md) — 七步 + 回滚 + ⚠️ DNS 归属的互斥选择 |
| **别的项目**怎么部署它 | [cloudflare/terraform/modules/worker/](cloudflare/terraform/modules/worker/README.md) — 可复用子模块的消费方契约（关键：构建产物由调用方生成） |
| 钉着某个 tag，该不该动 ref | [CHANGELOG.md](CHANGELOG.md) — 已发布版本之间的差异 + 版本口径 |

## 形态

一份 `axum::Router`，两个编译目标：本地 `tokio` 驱动拿秒级反馈循环，线上编到
`wasm32-unknown-unknown` 跑在 Cloudflare Workers 上，逻辑只有一份。
两个入口都已就位并各自过了 wasm32 / 宿主的编译门禁；✅ 线上那一份 2026-08-23 已部署。

内容侧：TOML 条目在构建期反序列化成强类型 `Catalog`，分类法字段用 newtype 而非
`String`，引用写错一个字母就是构建失败，而不是静默生成一个空页面。
wasm32 上没有文件系统，所以内容由 `build.rs` 在构建期内嵌 ——
线上与本地用的是同一张表，「本地看到的」与「线上渲染的」内容不可能不一致。

数据抓取全部关在构建期，`crates/site` 的库代码零 I/O。这条硬约束同时满足 wasm32 的
编译要求、保护 Workers 的 CPU 预算，并白送一个「退回纯静态站」的逃生舱 ——
那个逃生舱现在是实物：`xtask dump-html` 渲出全站 185 页，每次 CI 都跑。
逐条论证见 [docs/decisions/](docs/decisions/README.md)。

## 开发

```sh
.githooks/install                # 启用提交/推送门禁；每次新 clone 都要跑一次
cargo run -p xtask -- build-site # 静态导出 + Pagefind 索引（站内搜索要它）
cargo run -p dev                 # 本地浏览 → http://127.0.0.1:8080
cargo run -p xtask -- validate   # 内容五类校验（写错 vendor 或跨域引用分类都会硬失败）
```

`validate` 顺带按域打印条目数 —— 覆盖不均是常态，但必须是打印出来的事实，
而不是需要有人去数的印象。

改了 `content/` 下的 TOML 直接重跑 `cargo run -p dev` 即可（`build.rs` 会因
`rerun-if-changed` 重新内嵌）。只有**搜索索引**需要重跑 `build-site` ——
没跑过时搜索框不出现（不是出现一个点不动的框）。

其余子命令：`dump-html`（全站渲成 `dist/`，纯静态逃生舱）、`render-diff`（两条内容路径
逐字节比对）、`fetch`（抓上游活跃度，需 `GITHUB_TOKEN`）。产物目录的分工见
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md#构建管线) —— ⚠️ `dist/` 绝不能当资源目录。

`pre-commit` 查密钥、文件卫生与文档规则（毫秒级）；`pre-push` 再加上 `cargo fmt` /
`clippy` / `test`、`cargo check --target wasm32-unknown-unknown`（`site` 与 `edge` 各一条），
以及 `xtask validate` 与 `xtask render-diff`。
CI（[.github/workflows/ci.yml](.github/workflows/ci.yml)）跑的是同一套 —— 钩子只是
「别等 CI 才红」，不是唯一防线。详见 [.githooks/README.md](.githooks/README.md)。

## 授权

代码与内容**分开授权**，分界看目录：

| 范围 | 许可证 |
|------|--------|
| `crates/` `cloudflare/` `.github/` `.githooks/` | MIT **或** Apache-2.0，二选一 |
| `content/` `docs/` 与仓库根的说明文档（本文、[CHANGELOG.md](CHANGELOG.md)）| [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)（署名） |
| `content/generated/` | 从上游公开 API 抓来的事实性数据，不主张权利 |

条目文本与（段 3 之后的）FieldNote 是这个站的实质产出，所以要求署名而不是
放任整体搬走。完整说明见 [LICENSE](LICENSE)。
