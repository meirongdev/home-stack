# 内容模型

> 日期: 2026-08-22
> 状态: ✅ 生效事实（段 1 已完成；2026-08-22 加了域层）

内容模型的唯一真相源。[plans/2026-08-20-content-model.md](../plans/2026-08-20-content-model.md)
是**冻结的档案**（当时打算怎么做），本文是**现状**——两者不一致时以本文为准。

选型理由不在这里：分类法为什么用 newtype 见
[decisions/typed-content-model-not-hugo.md](../decisions/typed-content-model-not-hugo.md)，
为什么加一层域见
[decisions/domain-layer-not-flat-categories.md](../decisions/domain-layer-not-flat-categories.md)。

## 文件布局

```
content/
├── domains.toml      主题域声明（第一根轴，声明顺序即首页顺序）
├── categories.toml   域内分类声明（每条必须归属一个域）
├── vendors.toml      被替换的 SaaS 声明（第二根轴）
├── clusters.toml     FieldNote 用的集群标识
├── tools/*.toml      一个文件一条条目
└── generated/
    └── repo.json     xtask fetch 的产物，**committed**，不要手改
```

文件种类由**文件名**决定（`crates/site/src/load.rs` 按 basename 分派）：
根目录那四个是声明文件，`tools/` 下的是条目，`repo.json` 是生成数据。

## 三层分类法

| 层 | 文件 | 字段 | 基数 |
|---|------|------|------|
| 域 | `domains.toml` | `id` / `name` / `tagline` | 条目 **1 : 1**（单值必填） |
| 分类 | `categories.toml` | `id` / `name` / `domain` | 条目 **1 : N**（同域内） |
| vendor | `vendors.toml` | `id` / `name` | 条目 **1 : N**（与域正交） |

域是**单值必填**的：跨域的工具（Cilium 同时是 CNI、网络策略与流量可观测）只登记主域，
另一面写进 `detail`。这个取舍换来两件事 —— 「域内条目数」可加，面包屑有唯一父级；
代价见那份 ADR 的 Consequences。

## 条目字段

`crates/site/src/model.rs` 的 `Tool`。**非 `Option` 的字段就是硬要求**，
这是模型层唯一的表达手段，不要靠注释约束。

| 字段 | 类型 | 必填 | 说明 |
|------|------|:---:|------|
| `slug` | `Slug` | ✅ | 全局唯一，同时是 `/tools/{slug}` 的路径段 |
| `name` | `String` | ✅ | 展示名 |
| `summary` | `Summary` | ✅ | ≤125 字符（超了卡片撑破版） |
| `license` | `Spdx` | ✅ | SPDX 表达式；与上游检测值交叉核对 |
| `language` | `String` | ✅ | 实现语言 |
| `domain` | `DomainId` | ✅ | 主题域，单值 |
| `categories` | `Vec<CategoryId>` | | 必须全部属于本条目的 `domain` |
| `replaces` | `Vec<VendorId>` | | 基建类条目多半为空，那是正常的 |
| `self_host` | `Vec<DeployKind>` | | enum 而非自由文本，见下 |
| `hosted` | `bool` | ✅ | 上游是否也提供托管版 |
| `links.repo` | `Url` | ✅ | **没有出处的条目不该存在** |
| `links.site` / `links.docs` | `Url` | | |
| `detail` | `String` | ✅ | 架构形状、存储依赖、协议、部署形态 |
| `requires` | `Vec<String>` | | 跑起来之前要先准备什么 |
| `watch` | `Vec<String>` | | 上手前该知道的坑、锁定、许可证含义 |
| `upstream` | `Option<Upstream>` | | 只在「不写读者会踩坑」时出现（归档/停滞/易主） |
| `field_note` | `Option<FieldNote>` | | 一手运维证据，段 3 才落库 |

`DeployKind`：`SingleBinary` / `Docker` / `Compose` / `HelmChart` / `Operator` /
`Manifests`（裸 K8s 清单）/ `DebRpm` / `OsImage`（整机镜像 / ISO）/ `SourceBuild`。
`OsImage` 与 `Manifests` 是 2026-08-22 随 compute 域加的 ——
hypervisor 与不可变 K8s 发行版装的是操作系统，硬塞进 `DebRpm` 会让读者以为能 `apt install`。

`Upstream` 的 `as_of` 与 `source` 都**非 `Option`**：上游状态是会过期的判断，
不标核对日期与出处就没有可信度。

`FieldNote.decision` 也**非 `Option`**：每个数字都必须能追回一份出处文档。
⚠️ 这条设计有一个未决前提 —— homelab 仓库若不公开，那些 URL 在读者侧全是 404，
见 [ROADMAP.md](../ROADMAP.md) 开放项 7。

## 五类校验

`cargo run -p xtask -- validate`，全部**硬失败**。实现在 `crates/site/src/load.rs`，
报错带行列号与源码行（`toml::Spanned`）。

| # | 查什么 | 不查会怎样 |
|---|--------|-----------|
| 1 | `domain` / `categories` / `replaces` / `clusters` / `supersedes` 引用必须已声明 | 拼错一个字母，条目从所有导航里消失（Hugo 会静默生成一个空页面） |
| 2 | **跨域分类**：条目的每个分类都必须属于它的 `domain` | 两个引用各自都存在、只有组合是错的 —— 纯 `String` 分类法查不出这一类 |
| 3 | 孤儿分类 / 孤儿域：声明了但零条目引用 | 首页多出一张点进去是空页面的卡片 |
| 4 | `slug` 全局唯一 | 后者覆盖前者，或产生冲突路径 |
| 5 | `summary` ≤125 字符 | 卡片撑破版，只在视觉上暴露 |

外加一条生成数据的完整性检查：`repo.json` 的 key 必须是真实条目的 slug ——
条目改名后忘了重抓，页面上就会挂着一份对不上任何条目的数字。

`validate` 顺带按域打印条目数。覆盖不均是常态，但必须是**打印出来的事实**，
而不是需要有人去数的印象。

## 内容怎么进到两个编译目标里

```
content/*.toml ──┬─► crates/site/build.rs ─► OUT_DIR/content.rs ─► site::content::SOURCES
                 │        （构建期 include_str!，rerun-if-changed）      │
                 │                                                      ▼
                 │                                          site::content::catalog()
                 │                                                      │
                 │                                    ┌─────────────────┴─────────────────┐
                 │                                    ▼                                   ▼
                 │                             crates/dev [native]              crates/edge [wasm32]
                 │
                 └─► xtask read_sources()（磁盘）─► validate / render-diff 的对照侧
```

- **线上唯一的数据来源是内嵌表**：`wasm32-unknown-unknown` 上没有文件系统。
- `crates/dev` 也用内嵌表，所以「本地看到的」与「线上渲染的」内容不可能不一致。
  改了 TOML 靠 `cargo:rerun-if-changed` 触发重编，`cargo run -p dev` 再跑一次即生效。
- ⚠️ **`crates/site` 的库代码仍然零 I/O**。唯一的 I/O 在 `build.rs` 里，构建脚本不进产物。
- `xtask` 仍然从磁盘读一份 —— 那是 `validate` 精确报错的需要，也是
  `render-diff` 的对照侧（见 [ARCHITECTURE.md](../ARCHITECTURE.md#构建管线)）。

## 加内容的顺序

**先写够条目，再声明域。** 孤儿域是硬失败,零条目的域点进去是一张空页面。
同理，声明一个分类之前先确认至少有一条条目会引用它。

收录判据（`homelab 读者真会问它 ∧ 作者能对它说出四态之一`，缺一即不收）与
明确不收的类别见 [ROADMAP.md](../ROADMAP.md#明确不做)。
