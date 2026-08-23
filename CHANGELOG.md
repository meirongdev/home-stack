# Changelog

> 这份文件只回答一个问题：**你钉着 `?ref=<tag>` 消费
> [`cloudflare/terraform/modules/worker`](cloudflare/terraform/modules/worker/README.md)，
> 要不要动那个 ref。**
>
> 「现在是什么样」不在这里 —— 那在 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) 与
> [docs/reference/](docs/reference/README.md)；「还剩什么没做」在
> [docs/ROADMAP.md](docs/ROADMAP.md)。本文**只追加、不改写**已发布的条目。

## 版本口径

☠️ **内容与代码在同一个仓库**，所以一个 tag 钉住的是「模块 + 那一刻的目录内容」。
这跟一般的库不同：升 ref 会同时换掉你部署出去的**内容**。

| 变化 | 位次 |
|---|---|
| `modules/worker` 的输入契约破坏性变化（变量改名 / 删除 / 新增必填 / 默认值改变） | MAJOR |
| 新增可选输入、新增能力；目录内容的批量变化（新增域、成批条目） | MINOR |
| 修 bug、内容小修、文档 | PATCH |

📌 上游活跃度快照（`content/generated/repo.json`）每夜由机器人提交，**不单独发版** ——
它变了不代表你要动 ref。

## [Unreleased]

### Changed

- **部署方式**：state 从工作站本地文件迁到 Cloudflare R2，部署交给
  `.github/workflows/deploy.yml`（手动触发）。
  **对消费者无影响** —— `modules/worker` 本身不含 provider、不含 backend，
  你的根模块管你自己的 state。

### Fixed

- 文档里 `?ref=<tag>` 的占位符换成真实 tag，并写清一脚：消费方有**两处** `ref`
  （`source` 里那个决定用哪份 `.tf`，checkout home-stack 那个决定构建出哪份内容与
  wasm），两处必须是同一个 tag。写岔了不会报错，只会静默部署
  「A 版的基础设施 + B 版的内容」。

## [0.1.0] — 2026-08-23

首个可钉版本。在此之前只能钉 `main`，也就是「内容变了、你没改任何东西，
你的部署也变了」。

### Added

- `cloudflare/terraform/modules/worker`：**不含 provider、不含 backend**的可复用子模块 ——
  Worker + 不可变版本 + deployment（100% 流量）+ 可选的 custom domain 或 route
- 目录内容 **97 条 / 10 个域**；站内搜索（Pagefind，只索引工具页、按域 facet）
- 纯静态逃生舱：`xtask dump-html` → 185 页 + `404.html`，任何静态托管都能直接上
- 未命中路径返回**真 404**（不是 soft 404 的 200）

### 注意（消费方）

- ☠️ **模块不构建，调用方构建。** 远程 `source` 只给你 `.tf` 文件 —— wasm 与资源层
  不在里面，也不可能在里面。构建命令与 CI 片段见
  [模块 README](cloudflare/terraform/modules/worker/README.md#契约模块不构建调用方构建)
- ⚠️ `plan` 阶段就会 `lstat` 构建产物目录：没先构建，plan 直接失败（有意的）
- 实测包体 **346 KiB gz**（Cloudflare Workers Free 上限的 11%）
