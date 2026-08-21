# Plans

> 日期: 2026-08-22
> 带日期的方案与规格档案，**写完即冻结**。想知道「现在是什么样」不要读这里 ——
> 看 [../ROADMAP.md](../ROADMAP.md#现状)。

命名 `YYYY-MM-DD-<topic>.md`。本目录扁平，不分类别（单应用仓库，homelab 那套六类
`plans/<类别>/` 在这里是过度结构）。

| 方案 | 状态 | 结论 |
|------|------|------|
| [2026-08-20-content-model](2026-08-20-content-model.md) | ✅ 已完成（段 1） | 条目 schema、分类法、FieldNote 结构、`xtask validate` 的四类校验 |
| [2026-08-20-data-pipeline](2026-08-20-data-pipeline.md) | 🟡 GitHub 侧已完成；Prometheus 侧待实施 | `xtask fetch`（GitHub GraphQL + homelab Prometheus）+ 夜间 CI；⚠️ fetch 必须 fail-soft |

## 一份方案写完之后

方案冻结，不再随实现改动。实现落地后，**「现在是什么样」应另建 `reference/` 文档**，
方案只作为「当时为什么这么打算」的档案保留。
