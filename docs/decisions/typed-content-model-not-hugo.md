# 内容模型用 Rust 类型而非 Hugo 分类法：把引用错误变成编译错误

> 日期: 2026-08-20
> 状态: ✅ 采纳

## Context

用 Hugo 生成的静态站，条目字段（license / 语言 / stars / freshness /
categories / replaces / self-host 方式 / hosted）全靠 front matter + Hugo taxonomy。

Hugo 的 taxonomy 是**开放集合**：`replaces: datadog` 只是一个字符串。写错一个字母，
Hugo 会安静地生成一个空的 `/replaces/datadg/` 页面，站点照常构建、照常上线，
直到有人点进去才发现。这类错误在纯内容站里是最常见也最难自查的一种 ——
它不报错，只是悄悄产出一个死页面。

同类的还有三种：孤儿分类（声明了但没有条目引用）、重复 slug、
`summary` 超长把卡片撑破版（~125 字符的实际约束）。
在 Hugo 上这四种全部只能靠人眼。

## Options

| 方案 | 结论 |
|---|---|
| **TOML 条目 → serde 强类型 `Catalog`，构建期校验引用完整性** | ✅ 采纳 |
| Hugo + 自写 lint 脚本 | ⚠️ 能覆盖，但校验逻辑与渲染逻辑分家，容易漂移 |
| Hugo 原样照搬 | ❌ 放弃了换语言的主要回报 |
| JSON Schema 校验 | ❌ 只能查形状，查不了跨文件引用（vendor 是否已声明） |

## Decision

条目写 TOML，反序列化成强类型结构 —— **分类法字段用 newtype 而非 `String`**：

- `categories: Vec<CategoryId>` —— 必须存在于 `categories.toml`
- `replaces: Vec<VendorId>` —— 必须存在于 `vendors.toml`
- `self_host: Vec<DeployKind>` —— enum，不是自由文本
- `summary: Summary` —— newtype，校验 ≤125 字符
- `license: Spdx`

`cargo run -p xtask -- validate` 在构建期兑现引用完整性，失败即构建失败。
字段表、schema 与报错形态见
[../plans/2026-08-20-content-model.md](../plans/2026-08-20-content-model.md)。

## Consequences

**得到**

- 上面四类事故（未声明引用 / 孤儿分类 / 重复 slug / summary 超长）从「靠人眼」变成
  「构建失败」。这是换掉 Hugo 最实在的一份回报，也基本是唯一一份 ——
  **不要用「Rust 更快」之类的理由为这个决策辩护**，静态站的渲染速度从来不是瓶颈。
- 模板侧同样受益：Maud 是编译期检查的 HTML 宏，标签闭合与转义在 `cargo build` 就抓。

**付出**

- 加一个 vendor 或 category 要改两处（声明文件 + 条目）。这正是想要的摩擦。
- 内容贡献门槛高于 Hugo front matter —— 单人维护的站点里这不构成问题，
  一旦开放外部投稿需重评。

**边界**

`validate` 必须**硬失败**（内容错误本来就该拦住）；而
[数据管道](../plans/2026-08-20-data-pipeline.md)里的 `fetch` 必须 **fail-soft** ——
两者性质不同，别混为一谈。
