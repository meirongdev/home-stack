# Roadmap

> 日期: 2026-08-22
> 状态: 🚧 段 1 与段 4 的 GitHub 侧已完成；段 2 / 3 待实施
> 本文只回答两件事：**还剩什么没做**，和**明确不做什么**。
> 实施细节不写在这里 —— 每条都链到 [decisions/](decisions/README.md)（取舍）或
> [plans/](plans/README.md)（规格）。

> ⚠️ **编号是稳定标识，不是序号。** 关闭一条不重新编号、也不把号让给新条目：
> 空档就是「这号已经关掉了」的证据。

## 现状

**已生效**（段 1 + 段 4 的 GitHub 侧）：

- `crates/site` 模型 + Maud 模板 + Router（零 I/O）、`crates/dev` → `cargo run -p dev` 上 `:8080`
- `xtask validate`：引用未声明 / 孤儿分类 / slug 重复 / summary 超长，四类硬失败
- `xtask fetch`：从条目的 `links.repo` 一次 GraphQL 抓 stars / pushed_at / latest_release /
  license，产出 committed 的 `content/generated/repo.json`；工具页显示这些数字与 `fetched_at`，
  许可证与上游交叉核对是常设检查
- 29 条目录条目，收录范围限定在 homelab（判据见「明确不做」）

**未生效**：段 2（上边缘 + CI 双门禁）、段 3（FieldNote）、段 4 的 Prometheus 侧与夜间 CI。
线上部署还不存在。下一步是段 2。

## 四段实施

每段都能独立停下，段与段之间有明确出口判据。

| 段 | 内容 | 出口判据 |
|---|------|---------|
| **1** | ✅ 已完成，见「现状」。规格见 [plans/2026-08-20-content-model.md](plans/2026-08-20-content-model.md) | 已验证 |
| **2** | 上边缘。加 `crates/edge`、`wrangler.jsonc`、Pagefind 索引；`cloudflare/terraform` 加一条 DNS 记录。**同时建 CI 与两条门禁**：`cargo check --target wasm32-unknown-unknown`（依赖是否编得到 wasm32，不靠人记）与 `xtask render-diff`（两个目标逐字节比对，不一致即硬失败）—— 两条都是 [decisions/dual-target-axum.md](decisions/dual-target-axum.md) 承诺外包给 CI 的纪律，不能只停在 ADR 里 | 公网可访问；`crates/dev` 与线上渲染出**同一份 HTML**，且这条由 `render-diff` 在每次合并跑，不是实施时手工对比一次（这是判据不是「顺便看看」） |
| **3** | 一手证据层。`FieldNote` 四态（Running/Retired/Rejected/Evaluating）+ 按状态筛选的视图。目标 10 条，全部取自 homelab 已有文档 —— [decisions/field-notes-as-differentiator.md](decisions/field-notes-as-differentiator.md) 已点名 7 条可直接成稿，余 3 条实施时从 homelab `docs/records/*` 里挑 | 每条 FieldNote 的数字都能点回一份 decision/record 文档 |
| **4** | 数据管道自动化。`xtask fetch` + GitHub Actions + Tailscale。规格见 [plans/2026-08-20-data-pipeline.md](plans/2026-08-20-data-pipeline.md) | 连续 7 天夜间构建全绿，且**人为断开 tailnet 时构建仍然成功** |

段 1 与段 2 之间有天然止损点：若双目标编译在某个依赖上出意外，
`crates/dev` 加一个遍历路由 dump HTML 的子命令就退回纯静态站，照样能上 Cloudflare ——
只是失去 HTMX 那两个服务端交互。这个逃生舱是零 I/O 约束的副产品，不需要额外设计
（见 [decisions/dual-target-axum.md](decisions/dual-target-axum.md)）。

## 开放项

| # | 项目 | 说明 |
|---|------|------|
| 1 | **域名未定** | 建议 `obs.meirong.dev`。定下来才能写 `cloudflare/terraform` 那条 DNS 记录（段 2 前置） |
| 3 | **playbook 做不做** | 是否每个 vendor 一份分阶段迁移指南。⚠️ 作者没做过这些迁移，硬写会违反「每个数字都要有出处」的纪律。倾向先不做，段 3 之后按实际经历补。见 [plans/2026-08-20-content-model.md](plans/2026-08-20-content-model.md#未决) |
| 4 | **Prometheus 侧数据源是否值得** | 只服务少数几条 footprint，却要引入跨仓库 ACL 依赖（开放项 5）。等段 3 的 FieldNote 实际数量出来再定。见 [plans/2026-08-20-data-pipeline.md](plans/2026-08-20-data-pipeline.md#未决) |
| 5 | **Tailscale ACL 跨仓库依赖** | 段 4 需要 homelab `tailscale/terraform` 放行 `tag:ci` 到中枢 Prometheus。⚠️ 两边都要留注释 —— 日后清 ACL 打断它的症状是「站点数字停止更新」，不会有人立刻发现 |
| 6 | **首份 `reference/` 文档** | ⏰ 已到期：段 1 完成后内容模型已是生效事实，`docs/reference/content-model.md` 该建了 —— `plans/` 那份是冻结档案，不是现状（R1：建议 ≠ 事实） |
| 7 | **homelab 仓库是否公开** | ⚠️ `FieldNote.decision` 是 `Url` 且**非 `Option`**，「每个数字都能追回出处」是本站立身之本。若 homelab 是私有仓库，读者点进去全是 404 —— 护城河在**读者侧不可验证**，等于没有。若不公开就得改字段设计（引用 + 摘录，而非裸 URL）。段 3 前置。见 [decisions/field-notes-as-differentiator.md](decisions/field-notes-as-differentiator.md)、[plans/2026-08-20-content-model.md](plans/2026-08-20-content-model.md) |
| 8 | **缺一篇「为什么是 Rust」的 ADR** | 5 条 ADR 论证了落点、双目标、内容模型、差异化、SSR 值不值，唯独最底层的语言选择是公理：[decisions/cloudflare-workers-not-pages.md](decisions/cloudflare-workers-not-pages.md) 把「Axum + Maud + HTMX」当既定前提带过，而 [decisions/typed-content-model-not-hugo.md](decisions/typed-content-model-not-hugo.md) 还主动禁掉了「Rust 更快」这个理由却没给替代。真实理由可能很朴素（想练 / 工具链已有 / Pagefind 本身就是 Rust）—— 但按本仓库「把隐假设显式化」的标准，该写出来。**理由待作者本人填，不代拟** |
| 9 | **项目定名** | 三个名字在流通：`docs/README.md` H1 的「Probe Directory」、仓库目录名 `observability_directory`、拟用域名 `obs.meirong.dev`。与开放项 1 一起定 |
| 10 | **LICENSE 未定** | 仓库要公开（站点本身就是公开的），但根目录没有 LICENSE。内容与代码可能需分开授权 —— 常见做法是代码 MIT/Apache-2.0、条目文本与 FieldNote 用 CC BY 4.0 |

## 明确不做

| 不做什么 | 为什么 |
|---------|--------|
| 部署到 oracle-k3s | 六项运维负担（pod/告警/monitor/Trivy/requests/镜像流水线），而 oracle 已单向缩容到 2 OCPU/12GB。见 [decisions/cloudflare-workers-not-pages.md](decisions/cloudflare-workers-not-pages.md) |
| 运行时实时指标看板 | 会把公网站点绑上 homelab 可用性，且暴露内部拓扑。实测数字一律构建期抓取 |
| 用户账号 / 服务端持久化 shortlist | shortlist 走 localStorage（无服务端状态）。引入状态就等于引入数据库，整个零开销论证作废 |
| 编辑评分 / 星级排名 | 正是典型的 listicle 形态，且无证据支撑 |
| 为没跑过的工具编造 FieldNote | 覆盖率不均是诚实的。没有 FieldNote 的条目退化成元数据条目，那没什么不好 |
| 收录企业规模 / 团队流程向的条目（多租户横向扩展、告警工作流编排、企业 agent 平台） | 读者是 homelab（k3s、单人运维）。**收录判据：homelab 读者真会问它 ∧ 作者能对它说出四态之一，缺一即不收。**值班轮值这类团队流程工具只在能给出迁出路径时留（Grafana OnCall 上游已归档，页面价值在「原先靠它的手机推送怎么迁」）。⚠️ 「太重跑不动」**不是**不收的理由 —— 那正是 `Rejected` 最该写的一页（Sentry / Graylog / Thanos 因此保留） |
