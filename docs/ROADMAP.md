# Roadmap

> 日期: 2026-08-23
> 状态: 🚧 段 1 已完成；段 2 **已部署**（2026-08-23，`stack.meirong.dev`，出口判据「公网可访问」已兑现）；
> 段 4 的 GitHub 侧已完成（含夜间 CI）；段 3 待实施
> 本文只回答两件事：**还剩什么没做**，和**明确不做什么**。
> 实施细节不写在这里 —— 每条都链到 [decisions/](decisions/README.md)（取舍）或
> [plans/](plans/README.md)（规格）。

> ⚠️ **编号是稳定标识，不是序号。** 关闭一条不重新编号、也不把号让给新条目：
> 空档就是「这号已经关掉了」的证据。

## 现状

**已生效**（段 1 + 段 4 的 GitHub 侧）：

- `crates/site` 模型 + Maud 模板 + Router（零 I/O）、`crates/dev` → `cargo run -p dev` 上 `:8080`
- `xtask validate`：引用未声明（含域） / 跨域分类 / 孤儿分类与孤儿域 / slug 重复 /
  summary 超长，五类硬失败；顺带按域打印条目数
- `xtask fetch`：从条目的 `links.repo` 一次 GraphQL 抓 stars / pushed_at / latest_release /
  license，产出 committed 的 `content/generated/repo.json`；工具页显示这些数字与 `fetched_at`，
  许可证与上游交叉核对是常设检查。⚠️ 两条条目的上游不在 GitHub/GitLab（Forgejo 在 Codeberg、
  Proxmox VE 在自建 git），fetch 会跳过它们并告警 —— 这两条页面上没有活跃度数字
- **97 条目录条目，分布在 10 个域**（compute 7 / networking 12 / storage 9 / gitops 11 /
  secrets 6 / identity 6 / observability 29 / security 6 / iac 6 / data 5）。
  域层设计见 [decisions/domain-layer-not-flat-categories.md](decisions/domain-layer-not-flat-categories.md)，
  收录范围仍限定在 homelab（判据见「明确不做」）

**已生效**（段 2 的代码侧 + 段 4 的夜间 CI）：

- `crates/site/build.rs` 构建期把 `content/` 内嵌成一张表 —— wasm32 没有文件系统，
  这是线上唯一的数据来源；`crates/dev` 用的也是它，「本地看到的」与「线上渲染的」
  内容不可能不一致
- `crates/edge`（`worker` 0.8.5 + `#[event(fetch)]`，约 60 行）与 `crates/edge/wrangler.jsonc`
- `xtask dump-html`（185 页 + 404.html → `dist/`，纯静态逃生舱从设想变成实物）、
  `xtask render-diff`、`xtask build-site`（Pagefind 1.5.2 索引 → `public/`）
- **站内搜索**：Pagefind 只索引 97 个工具页（列表页不进索引），按域做 facet
- **CI 存在了**：`.github/workflows/ci.yml` 跑 9 道门禁，含 ADR 承诺外包给 CI 的
  两条红线（wasm32 编译约束 ×2、render-diff）；`nightly-fetch.yml` 每夜刷上游活跃度

**未生效**：段 3（FieldNote）、段 4 的 Prometheus 侧、HTMX 与 calculator / advisor。
✅ **已部署**（2026-08-23）：<https://stack.meirong.dev>。`cloudflare/terraform/` 建了 4 个资源
（`cloudflare_worker` + `worker_version` + `workers_deployment` + `workers_custom_domain`，
provider 5.23.0），实测包体 **346 KiB gz = Free 上限的 11%**。DNS 归属选了**方案 A**
（Cloudflare 自建 `AAAA 100::` 占位记录并签证书），homelab 那边只留注释、不声明第二份。
⚠️ 首次 apply 撞过一个模块 bug —— custom domain 缺 `depends_on`，与「上传资源层要十几秒」的
版本资源并发，撞 `400 / 100124 Worker has no deployments`；已修并写进
[runbooks/deploy-cloudflare.md](runbooks/deploy-cloudflare.md) 第 5 步。
✅ **部署已经交给 CI**（2026-08-23）：state 迁入 R2
（`terraform-backend/home-stack/cloudflare.tfstate`），`deploy.yml` 在干净 runner 上跑通
两次 —— 工作站不再是唯一能部署的机器。⚠️ 验收判据不是绿灯、也不是 `No changes`；它和
「apply 的先建后毁顺序」一起记在
[runbooks/deploy-cloudflare.md](runbooks/deploy-cloudflare.md) 第 7 步。
✅ **可钉版本**：`v0.1.0`（2026-08-23，仓库首个 tag，指向上面这个已验证的状态）。
外部项目按 `?ref=v0.1.0` 消费 `modules/worker`，不再只能钉 `main` —— 契约与
「两处 `ref` 必须同一个 tag」这一脚，见
[modules/worker/README.md](../cloudflare/terraform/modules/worker/README.md)。

## 四段实施

每段都能独立停下，段与段之间有明确出口判据。

| 段 | 内容 | 出口判据 |
|---|------|---------|
| **1** | ✅ 已完成，见「现状」。规格见 [plans/2026-08-20-content-model.md](plans/2026-08-20-content-model.md) | 已验证 |
| **2** | 🟡 **已部署**（2026-08-23，`stack.meirong.dev`）。已有：`crates/edge`、构建期内容内嵌、`dump-html` / `render-diff` / `build-site`、Pagefind 索引与搜索、CI 九道门禁、`cloudflare/terraform`（4 个资源已 apply）、远端 state（R2）、CI 部署（`deploy.yml` 干净 runner 首跑通过）。未有：render-diff 的 wasm 运行时那一半（开放项 13） | 公网可访问 ✅（2026-08-23 实测：首页、条目页、域页 12 张卡、`pagefind/pagefind-ui.js` 全 200）；`render-diff` 每次合并跑 ✅（但只覆盖内容路径，不覆盖 wasm 运行时 —— 判据只兑现了一半） |
| **3** | 一手证据层。`FieldNote` 四态（Running/Retired/Rejected/Evaluating）+ 按状态筛选的视图。目标 10 条，全部取自 homelab 已有文档 —— [decisions/field-notes-as-differentiator.md](decisions/field-notes-as-differentiator.md) 已点名 7 条可直接成稿，余 3 条实施时从 homelab `docs/records/*` 里挑。⚠️ 分母已从 29 变成 97，「10 条」这个目标该怎么摊到域上是开放项 11 | 每条 FieldNote 的数字都能点回一份 decision/record 文档 |
| **4** | 🟡 GitHub 侧已完成（`xtask fetch` + `nightly-fetch.yml` 每夜刷新并自动提交）。Prometheus 侧**有意不做** —— 见开放项 4：它只服务 FieldNote 的 footprint，而 FieldNote 现在是 0 条 | 连续 7 天夜间构建全绿（待观察）；「断开 tailnet 仍然成功」这条**当前空成立** —— 根本没有 tailnet 依赖 |

段 1 与段 2 之间那个天然止损点**已经变成实物**：`xtask dump-html` 把全站渲成
`dist/`（185 页 + 404.html，含同一份 Pagefind 索引），任何静态托管都能直接上，
只是失去 HTMX 那两个服务端交互。它每次 CI 都跑，所以是一条**验证过的**退路，
不是一句设想（见 [decisions/dual-target-axum.md](decisions/dual-target-axum.md)）。

## 开放项

| # | 项目 | 说明 |
|---|------|------|
| 3 | **playbook 做不做** | 是否每个 vendor 一份分阶段迁移指南。⚠️ 作者没做过这些迁移，硬写会违反「每个数字都要有出处」的纪律。倾向先不做，段 3 之后按实际经历补。见 [plans/2026-08-20-content-model.md](plans/2026-08-20-content-model.md#未决) |
| 4 | **Prometheus 侧数据源是否值得** | 只服务少数几条 footprint，却要引入跨仓库 ACL 依赖（开放项 5）。等段 3 的 FieldNote 实际数量出来再定。见 [plans/2026-08-20-data-pipeline.md](plans/2026-08-20-data-pipeline.md#未决) |
| 5 | **Tailscale ACL 跨仓库依赖** | 段 4 需要 homelab `tailscale/terraform` 放行 `tag:ci` 到中枢 Prometheus。⚠️ 两边都要留注释 —— 日后清 ACL 打断它的症状是「站点数字停止更新」，不会有人立刻发现 |
| 7 | **homelab 仓库是否公开** | ⚠️ `FieldNote.decision` 是 `Url` 且**非 `Option`**，「每个数字都能追回出处」是本站立身之本。若 homelab 是私有仓库，读者点进去全是 404 —— 护城河在**读者侧不可验证**，等于没有。若不公开就得改字段设计（引用 + 摘录，而非裸 URL）。段 3 前置。见 [decisions/field-notes-as-differentiator.md](decisions/field-notes-as-differentiator.md)、[plans/2026-08-20-content-model.md](plans/2026-08-20-content-model.md) |
| 8 | **缺一篇「为什么是 Rust」的 ADR** | 6 条 ADR 论证了落点、双目标、内容模型、差异化、SSR 值不值、域层，唯独最底层的语言选择是公理：[decisions/cloudflare-workers-not-pages.md](decisions/cloudflare-workers-not-pages.md) 把「Axum + Maud + HTMX」当既定前提带过，而 [decisions/typed-content-model-not-hugo.md](decisions/typed-content-model-not-hugo.md) 还主动禁掉了「Rust 更快」这个理由却没给替代。真实理由可能很朴素（想练 / 工具链已有 / Pagefind 本身就是 Rust）—— 但按本仓库「把隐假设显式化」的标准，该写出来。**理由待作者本人填，不代拟** |
| 11 | **FieldNote 目标怎么摊到 10 个域** | 段 3 的「10 条」是对着 29 条可观测条目定的，现在分母是 97。两种表述：「先把可观测那一域做满」（覆盖率集中、看得出深度）或「每个域至少 1 条」（广度好看、每域都浅）。⚠️ 无论选哪个，都不能为没跑过的工具编造 FieldNote —— 见 [decisions/domain-layer-not-flat-categories.md](decisions/domain-layer-not-flat-categories.md) 的 Consequences。段 3 前置 |
| 12 | **跨域工具只登记主域，读者会找不到** | Cilium 记在网络域，从「安全加固」域里找不到它；Harbor 的漏洞扫描、Tetragon 与 Cilium 的关系同理。当前缓解只有 `detail` 里的一句话，不是机制。可选解法：域页面加一块「相关但登记在别处」的手写交叉引用（又一个要维护的分类法），或等 Pagefind 搜索（段 2）上线后认定「搜得到就够了」。⚠️ 先别急着改模型 —— 多值域已在 ADR 里被否决过 |
| 13 | **render-diff 只兑现了一半** | [decisions/dual-target-axum.md](decisions/dual-target-axum.md) 承诺「两个目标逐字节比对」。现在比的是**两条内容路径**（磁盘 vs 构建期内嵌），覆盖「改了 TOML 内嵌表没跟上」这类漂移；**没有**比 wasm32 运行时的输出。补法：CI 里 `worker-build` + `wrangler dev` 起一个本地 Worker，用同一份 `all_paths()` 逐条 curl 再和 `dist/` 比对。⚠️ 在补上之前，不要把那条 ADR 的判据当已满足 |

## 明确不做

| 不做什么 | 为什么 |
|---------|--------|
| 部署到 oracle-k3s | 六项运维负担（pod/告警/monitor/Trivy/requests/镜像流水线），而 oracle 已单向缩容到 2 OCPU/12GB。见 [decisions/cloudflare-workers-not-pages.md](decisions/cloudflare-workers-not-pages.md) |
| 运行时实时指标看板 | 会把公网站点绑上 homelab 可用性，且暴露内部拓扑。实测数字一律构建期抓取 |
| 用户账号 / 服务端持久化 shortlist | shortlist 走 localStorage（无服务端状态）。引入状态就等于引入数据库，整个零开销论证作废 |
| 编辑评分 / 星级排名 | 正是典型的 listicle 形态，且无证据支撑 |
| 为没跑过的工具编造 FieldNote | 覆盖率不均是诚实的。没有 FieldNote 的条目退化成元数据条目，那没什么不好 |
| 收录企业规模 / 团队流程向的条目（多租户横向扩展、告警工作流编排、企业 agent 平台） | 读者是 homelab（k3s、单人运维）。**收录判据：homelab 读者真会问它 ∧ 作者能对它说出四态之一，缺一即不收。**值班轮值这类团队流程工具只在能给出迁出路径时留（Grafana OnCall 上游已归档，页面价值在「原先靠它的手机推送怎么迁」）。⚠️ 「太重跑不动」**不是**不收的理由 —— 那正是 `Rejected` 最该写的一页（Sentry / Graylog / Thanos / Rook-Ceph / Keycloak 因此保留） |
| 收录「自托管应用」本身（媒体库、RSS、笔记、相册、密码管理器） | 那是另一个目录。本站收的是**跑这些应用所需的底座**（计算、网络、存储、交付、密钥、身份、可观测、安全、IaC、数据）。判据同上一行：作者对 Jellyfin / Miniflux 说得出四态，但那些条目回答的是「我想看电影」而不是「我该选哪套底座」，混进来会让每个域的候选集失去可比性 |
| 新增域时先声明域、后写条目 | 孤儿域是硬失败（零条目的域点进去是一张空页面）。**先写够条目，再声明域** —— 这条顺序由 `xtask validate` 强制，不靠自觉 |
