# Probe Directory Docs

> 日期: 2026-08-22
> 这是**入口索引**。设计事实都在下面链接的文档里，本页不复制副本。

## 这个项目是什么

一个自托管可观测性工具目录，按「你要替换掉哪个 SaaS」索引，用 Rust 实现。
核心差异化是**一手运维证据**：每个工具带作者真实跑过的 FieldNote（见
[decisions/field-notes-as-differentiator.md](decisions/field-notes-as-differentiator.md)）。

⚠️ 每份记录文首的 `状态` 字段是唯一判据 —— 不要按「文档在这儿所以东西跑着」来读。
本目录里**已生效**与**仍是意图**的内容混在一起：生效的是段 1（`crates/site` / `dev` / `xtask`）
与段 4 的 GitHub 侧，上边缘与 FieldNote 都还没实施，见 [ROADMAP.md](ROADMAP.md#现状)。

## 从哪里开始

| 想知道什么 | 读这个 |
|-----------|--------|
| 整体长什么样 | [ARCHITECTURE.md](ARCHITECTURE.md) — 单页总览：双目标编译、路由分工、栈选型、已知约束 |
| 为什么是这个方案 | [decisions/](decisions/README.md) — 轻量 ADR，5 条 |
| 具体怎么建模 / 怎么抓数据 | [plans/](plans/README.md) — 带日期的规格，写完即冻结 |
| 还剩什么没做 | [ROADMAP.md](ROADMAP.md) — 开放项 + 四段实施顺序 + 明确不做 |

## 目录一览

| 位置 | 内容 | 收 | **不收** |
|------|------|-----|---------|
| 顶层 [ARCHITECTURE.md](ARCHITECTURE.md) / [ROADMAP.md](ROADMAP.md) | 现在打算长什么样 / 还剩什么没做 | **持续维护**，随决策变化更新 | 论证与步骤（只链不述，见下） |
| [decisions/](decisions/README.md) | 为什么选 A 不选 B | 选型场景、被否决的选项、取舍 | 怎么做（步骤） |
| [plans/](plans/README.md) | 当时打算怎么做 | 带日期的规格，**写完即冻结** | 需要长期维护的事实 |

`reference/` 目前仍不存在 —— 段 1 已完成，内容模型已是生效事实，按下方约定（**建议 ≠ 事实**）
现在就该建第一份，见 [ROADMAP.md](ROADMAP.md) 开放项 6。

## 文档约定

沿用 homelab 仓库的 R1–R3（目录归属 / 命名 / 文首必填字段）。**R2 / R3 由本地 git hook
机械检查**（`.githooks/checks/docs.py`），R1 是语义判断、仍然靠人守：

- **目录归属**：一篇文档只属于一类。需要「随架构变化持续更新」的进 `reference/`
  —— 顶层 ARCHITECTURE.md / ROADMAP.md 是这一类的既有例外（见上表）；
  「记录某一天的判断」的进 `plans/` 或 `decisions/`。**建议 ≠ 事实。**
- **命名**：`decisions/` 用描述性 kebab-case `<topic>.md`，日期写在文首、不靠文件名排序；
  `plans/` 用 `YYYY-MM-DD-<topic>.md`。
- **文首必填**：H1 必须是文件第一行，**字段名一律用中文**（`日期` / `状态`，不写
  `Last updated` / `Status`）。`decisions/` 要 `日期` + `状态` + Context / Options /
  Decision / Consequences；`plans/` 要 `日期` + `状态` + 结论；顶层 ARCHITECTURE.md /
  ROADMAP.md 同样要 `日期` + `状态`。各目录的 README 是导航页，只带 `日期`。
- **活文档只链不述**：ARCHITECTURE.md 与 ROADMAP.md 会被长期修改，遇到已被 ADR 论证过的
  内容，只写结论 + 链接，不复述论证。ADR 之间可以互相复述（各自冻结、必须自足），
  活文档不行 —— **复述的每一处都是未来的漂移点。**
- **决策被推翻时不删旧文件**：把文首状态改成 `已废弃`，并链到取代它的记录。

钩子还会查：全部相对链接与锚点、索引表与实际文件是否同步、「N 条 ADR」这类计数声明
是否与实际篇数一致（这三类都是真实发生过的漂移）。装法见 [../.githooks/README.md](../.githooks/README.md)。
