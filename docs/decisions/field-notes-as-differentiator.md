# 内容差异化靠 FieldNote 四态：Running / Retired / Rejected / Evaluating

> 日期: 2026-08-20
> 状态: ✅ 采纳

## Context

目录站每个条目给的是 license、语言、GitHub stars、freshness 徽章、
categories、replaces、self-host 方式。**这些字段 GitHub API 全都有** ——
任何人花一天就能抄出一个字段等价的站，这样的目录没有护城河。

而本项目的作者在两个 k3s 集群上真跑着其中相当一部分工具（Prometheus、Loki、Tempo、
OTel Collector、Sloth、OpenCost、Falco、Trivy Operator、Tetragon、Uptime Kuma、
Alertmanager、Grafana），homelab 仓库里已有成体系的一手记录：
`docs/reference/observability-*.md`、`docs/decisions/*.md`、`docs/records/*.md`。

这批材料是结构性稀缺的 —— **一个纯做目录的站永远拿不到它，因为它没有运行过任何东西**。

## Options

| 方案 | 结论 |
|---|---|
| **加 `FieldNote`，状态四态 Running / Retired / Rejected / Evaluating** | ✅ 采纳 |
| 只做一份字段齐全的目录 | ❌ 无差异化，做出来只是一个更慢的复制品 |
| 加「编辑评分 / 星级」 | ❌ 正是典型的 listicle 形态，且无证据支撑 |
| 加运行时实时指标看板 | ❌ 把公网站点绑上 homelab 可用性，且暴露内部拓扑 |

## Decision

条目可选带 `FieldNote`，其 `status` 是**四态而非一态**：

| 状态 | 含义 | 为什么稀缺 |
|------|------|-----------|
| `Running` | 我跑着，这是实测足迹和踩过的坑 | 目录站给不出资源足迹与告警盲区 |
| `Retired` | 我跑过，退役了，因为 —— 附接替者 | 几乎没有站点记录「什么被什么取代」 |
| `Rejected` | 我评估过，没上，因为 —— | **整个自托管内容生态近乎空白** |
| `Evaluating` | 在评，尚无结论 | 防止把「还没想清楚」冒充成结论 |

`Rejected` 是这四个里最有价值的。读者从「这工具存在」到「这工具适不适合我」之间，
缺的正是这一步，而没人写它 —— 因为写它要求你先认真评估过再放弃。

字段结构（`footprint` / `gotchas` / `blind_spots` / `supersedes` / `decision`）见
[../plans/2026-08-20-content-model.md](../plans/2026-08-20-content-model.md)。

### 已有素材（全部取自 homelab 仓库，可直接成稿）

| 条目 | 状态 | 内容 |
|------|------|------|
| Prometheus | Running | 12 GB VM 上 234,532 series 撞到 limit 88%；k3s 单进程让 kubelet 重复暴露 `apiserver_*`/`etcd_*`，近一半 series 是重复的或没人看的 |
| kube-prometheus-stack | Running·有陷阱 | 新增 `PrometheusRule`/`ServiceMonitor` 必须带 `release:` 标签，否则 operator selector **静默忽略** |
| Uptime Kuma | Running·有盲区 | 死人开关在**收发双方一起挂**时静默失明，实测 580s 缺口零翻转 |
| Trivy Operator | Running·有盲区 | Docker Hub 匿名配额打空后扫描 FATAL 且不自动重试，而告警**缺报告按 0 计入** |
| OTel Collector | Running | `container` operator 修掉 CRI 分段长行不重组（>16KB 日志被拆）；`file_storage` checkpoint 修掉重启丢整个停机窗口日志 |
| Gotify + gotify-bridge | Retired | Alertmanager 原生 `telegramConfigs` 让两个常驻组件一起消失 |
| Crossplane | Rejected | 认真评估后未采纳，理由已归档 |

## Consequences

**得到**

- 一条抄不走的护城河。字段可以被复制，运行经历不能。
- 站点获得一个结构上全新的浏览维度：按状态筛选，
  「有哪些东西被人试过之后放弃了」本身就是一个值得存在的页面。

**付出 / 纪律**

- ⚠️ **每个数字都必须能追回一份出处文档**（`decision` 字段指向 homelab
  `docs/decisions/*` 或 `docs/records/*`）。没有出处的经验之谈和 listicle 没有区别 ——
  而「不是 listicle」正是本站自己的立身之本，必须做得更实。
- FieldNote 会随 homelab 变化而过期。`Retired` 的条目**不删**，改状态并填 `supersedes` ——
  沿用 homelab RULES 的教训：**挂着「生产运行」的死文档比没有文档更坏**
  （Bifrost 那份误导了 5 天）。
- 覆盖率天然不均：只有作者跑过的工具有 FieldNote。这是诚实的，不要为了铺满而编造 ——
  没有 FieldNote 的条目就退化成普通元数据条目，那也没什么不好。

**明确不做**

不接运行时实时指标。实测数字在**构建期**抓取并 committed，理由见
[../plans/2026-08-20-data-pipeline.md](../plans/2026-08-20-data-pipeline.md)。
