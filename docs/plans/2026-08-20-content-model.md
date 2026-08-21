# 内容模型规格：条目 schema、分类法、FieldNote

> 日期: 2026-08-20
> 状态: ✅ 已完成（段 1）—— 本文冻结，是「当时打算怎么做」；
> 现行事实待建 `docs/reference/content-model.md`（[../ROADMAP.md](../ROADMAP.md) 开放项 6）
> 结论：条目写 TOML → serde 强类型 `Catalog`；分类法字段用 newtype，
> 引用完整性在 `xtask validate` 构建期兑现（硬失败）。
> 选型理由见 [../decisions/typed-content-model-not-hugo.md](../decisions/typed-content-model-not-hugo.md)
> 与 [../decisions/field-notes-as-differentiator.md](../decisions/field-notes-as-differentiator.md)。

## 目录布局

```
content/
├── vendors.toml            被替换的 SaaS 声明（datadog / splunk / sentry / ...）
├── categories.toml         信号分类声明（metrics / logs / traces / apm / errors / uptime / profiling / alerts / all-in-one）
├── clusters.toml           FieldNote 用的集群标识（homelab / oracle-k3s）
├── tools/<slug>.toml       每工具一份
├── playbooks/<vendor>.md   迁移 playbook 长文（Markdown，构建期渲染）
└── generated/              xtask fetch 产物，committed，见 2026-08-20-data-pipeline.md
    ├── repo.json           stars / pushedAt / latestRelease / licenseInfo
    └── footprint.json      来自 homelab Prometheus 的实测足迹
```

## Tool schema

条目字段（前 8 项）对齐常见目录条目，加上它没有的 `field_note`。

```rust
#[derive(Deserialize)]
pub struct Tool {
    pub slug:       Slug,
    pub name:       String,
    pub summary:    Summary,            // newtype: 校验 ≤125 字符
    pub license:    Spdx,
    pub language:   String,
    pub categories: Vec<CategoryId>,    // 必须存在于 categories.toml
    pub replaces:   Vec<VendorId>,      // 必须存在于 vendors.toml
    pub self_host:  Vec<DeployKind>,    // enum，不是自由文本
    pub hosted:     bool,               // 上游是否也提供托管版

    #[serde(default)]
    pub field_note: Option<FieldNote>,  // ← 差异化的一半
}

#[derive(Deserialize)]
pub enum DeployKind {
    SingleBinary, Docker, Compose, HelmChart, Operator, DebRpm, SourceBuild,
}
```

⚠️ `stars` / `freshness` **不在 schema 里** —— 它们来自 `content/generated/repo.json`，
在 `Catalog` 组装时 join 进来。手写的条目文件里不该出现会过期的数字。

## FieldNote schema

```rust
#[derive(Deserialize)]
pub enum Status { Running, Retired, Rejected, Evaluating }

#[derive(Deserialize)]
pub struct FieldNote {
    pub status:      Status,
    pub since:       Option<Date>,
    pub clusters:    Vec<ClusterId>,     // 必须存在于 clusters.toml
    pub footprint:   Option<Footprint>,  // requests / limits / 实测峰值
    pub gotchas:     Vec<Gotcha>,        // 一句话 + 可选证据链接
    pub blind_spots: Vec<String>,        // 它看不见什么
    pub supersedes:  Option<Slug>,       // Retired 时指向接替者
    pub decision:    Url,                // → homelab docs/decisions/* 或 docs/records/*
}
```

`decision` **不是 Option**。每个 FieldNote 都必须能追回一份出处文档 ——
这条纪律靠类型强制，不靠自觉（理由见
[../decisions/field-notes-as-differentiator.md](../decisions/field-notes-as-differentiator.md)）。

### 示例条目

```toml
# content/tools/prometheus.toml
slug       = "prometheus"
name       = "Prometheus"
summary    = "CNCF 毕业的指标事实标准。拉模型 + PromQL + 本地 TSDB。"
license    = "Apache-2.0"
language   = "Go"
categories = ["metrics", "alerts"]
replaces   = ["datadog", "new-relic"]
self_host  = ["SingleBinary", "Docker", "HelmChart", "Operator"]
hosted     = false

[field_note]
status      = "Running"
since       = "2026-03-01"
clusters    = ["homelab", "oracle-k3s"]
decision    = "https://github.com/<owner>/homelab/blob/main/docs/decisions/prometheus-series-reduction.md"
blind_spots = ["remote-write 断链时中枢看不见 oracle 侧，靠 absent() 规则兜底"]

[field_note.footprint]
mem_peak_7d = "2715Mi"
mem_limit   = "3072Mi"
note        = "12GB VM 上最大的单一内存消耗者；234,532 active series / 7,562 samples/s / TSDB 3.68 GiB"

[[field_note.gotchas]]
text = "k3s 把 apiserver 与 kubelet 跑在同一进程，kubelet 的 /metrics 因此把整个 legacyregistry 全吐出来——apiserver_* 与 etcd_* 被完整抓了第二遍，占该 job 全部 series 的 80%。"
```

## xtask validate — 四类校验

全部**硬失败**。这四类在 Hugo 上只能靠人眼，是换掉它的主要回报。

| # | 校验 | Hugo 上会怎样 |
|---|------|--------------|
| 1 | `replaces` / `categories` / `clusters` 引用必须已声明 | 静默生成一个空的 taxonomy 页面并正常上线 |
| 2 | 孤儿分类：声明了但零条目引用 | 生成一个空列表页 |
| 3 | slug 唯一 | 后者覆盖前者或产生冲突路径 |
| 4 | `summary` ≤125 字符 | 卡片被撑破版，只在视觉上暴露 |

报错形态（贴近 rustc，带 help / note）：

```
error: 未声明的 vendor 引用
  --> content/tools/vector.toml:8:14
   |
 8 |   replaces = ["datadog-logs"]
   |               ^^^^^^^^^^^^^^
   |
   = help: vendors.toml 已声明: datadog, datadog-apm, datadog-profiler, splunk, ...
   = note: Hugo 在这里会静默生成一个空的 /replaces/datadog-logs/ 页面并正常上线
```

## Markdown 渲染时机

playbook 长文与 FieldNote 里的多行文本，一律在**构建期**用 `pulldown-cmark`
渲成 HTML，随 `Catalog` 一起内嵌进 WASM。

⚠️ **绝不在 handler 里 parse Markdown。** Workers Free 档 CPU 是 10 ms/请求，
Maud 渲染是微秒级不构成压力，但运行时 Markdown 解析会把这份余量吃掉。

## 未决

- **playbook 的粒度**：是否每个 vendor 一份（4–14 周分阶段）。待定 ——
  作者没有做过这些迁移，写出来会违反「每个数字都要有出处」的纪律。
  倾向：先不做 playbook，段 3 之后按实际经历补。
- **`hosted` 字段的口径**：标「上游是否也卖托管版」。语义有点含糊
  （Grafana / Loki / Tempo 都是 true），实施时再定要不要保留。
- **calculator / advisor 的估算模型系数**：cost 估算与场景推荐的系数（SaaS 定价、
  本地资源成本、约束→排序映射）从哪来、怎么定，尚未设计。⚠️ 不能拍脑袋编，
  否则违反「数字要有出处」的纪律。方向见已采纳的
  [calculator-advisor-justify-ssr](../decisions/calculator-advisor-justify-ssr.md)：
  系数应尽量取自 FieldNote 实测（footprint），外部定价做显式假设标注、可追回出处。
  段 1 实施时定口径。
