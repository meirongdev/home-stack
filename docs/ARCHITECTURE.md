# Architecture — 一份 Axum Router，两个编译目标

> 日期: 2026-08-23
> 状态: 🟡 部分已完成 —— 段 1 全部、段 2 的代码全部（`crates/edge`、`wrangler.jsonc`、
> Pagefind、`dump-html` / `render-diff`、CI 双门禁）已就位，且 **2026-08-23 已部署到
> `stack.meirong.dev`**；HTMX 与 calculator / advisor 交互仍待实施

单页总览。选型理由不在这里，在 [decisions/](decisions/README.md)；
具体建模与数据抓取规格在 [plans/](plans/README.md)。

## 一句话

站点是**服务端渲染的**（Maud 模板 + HTMX 局部替换），但**没有常驻服务器** ——
同一份 `axum::Router` 编译到 `wasm32-unknown-unknown`，由 Cloudflare Workers 运行时驱动。
本地开发时另一个入口用 tokio 驱动同一个 Router，拿秒级反馈循环。

## Crate 布局

```
crates/
├── site/     纯逻辑，零 I/O —— 每一行都必须能编到 wasm32
│   ├── build.rs       构建期把 content/ 的 TOML include_str! 成一张表（唯一的 I/O，不进产物）
│   ├── content.rs     内嵌内容 → 已校验的 Catalog（线上唯一的数据来源）
│   ├── router.rs      axum::Router<&'static Catalog> + all_paths / render_path
│   ├── templates.rs   maud，编译期检查的 HTML
│   ├── model.rs       serde 强类型 Catalog（构建期已校验完）
│   └── load.rs        TOML → Catalog + 五类校验（入参是字符串，不碰文件系统）
├── dev/      native  —— tokio + axum::serve，cargo run → :8080（外加从 public/ 伺服资源）
├── edge/     wasm32  —— worker + #[event(fetch)]，worker-build → wrangler deploy
└── xtask/    native  —— 构建期工具：validate / fetch / dump-html / render-diff / build-site

cloudflare/terraform/       薄根模块：配 provider + backend，喂本仓库的构建产物路径
└── modules/worker/         可复用子模块（3 个核心资源 + 可选 custom_domain 或 route，无 provider / 无 backend）
.github/workflows/      ci（9 道门禁）/ nightly-fetch（段 4）/ deploy（手动触发，已跑通）
```

`crates/edge` 的依赖挂在 `[target.'cfg(target_arch = "wasm32")'.dependencies]` 下 ——
`worker` 编不到宿主平台，而 `cargo test --workspace` 与 `cargo clippy --all-targets`
是在宿主上跑的。宿主构建下它是个空 crate，wasm32 构建下才是真正的入口。
代价是**宿主门禁一行都查不到它**，所以 `cargo check -p edge --target wasm32-unknown-unknown`
是一条独立的门禁（CI 与 pre-push 各有一条）。

**`crates/site` 零 I/O 是硬约束，不是风格偏好。** 它同时兑现三件事：满足 wasm32
的编译约束、把渲染与数据抓取逼到构建期（运行时零 CPU 尖峰）、白送一个退回纯静态站的
逃生舱 —— 那个逃生舱现在是**实物**而不是设想：`xtask dump-html` 就是它，每次 CI 都跑。
逐条论证见 [decisions/dual-target-axum.md](decisions/dual-target-axum.md)。

⚠️ 唯一的例外是 `build.rs`：它在构建期读 `content/`，把 TOML `include_str!` 成一张表。
**构建脚本不进产物**，库代码仍然零 I/O。这是 wasm32 没有文件系统的必然结果 ——
详见 [reference/content-model.md](reference/content-model.md#内容怎么进到两个编译目标里)。

```
                    crates/site  （纯逻辑，零 I/O）
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
   crates/dev  [native]              crates/edge  [wasm32]
   tokio + axum::serve               worker + #[event(fetch)]
   cargo run  →  :8080               worker-build → wrangler deploy
```

两个入口都是薄封装：`crates/edge` 约 60 行（panic hook + 取 Catalog + 交给 Router），
`crates/dev` 约 100 行（tokio 起服务 + 从 `public/` 伺服 Pagefind 索引）。
路由与模板一行都不在入口里。理由见
[decisions/dual-target-axum.md](decisions/dual-target-axum.md)。

## 内容分类法：域 → 分类，另加一条 SaaS 轴

导航的第一根轴是**域**（`content/domains.toml`），域内再按**分类**
（`content/categories.toml`，每条声明归属域）收窄。第二根轴是「替换哪个 SaaS」
（`content/vendors.toml`），它与域并列但覆盖不到基建类条目 —— 所以不能当第一根用。

`Tool.domain` 单值必填，跨域分类是构建期硬失败。论证与代价（跨域工具只能登记主域）见
[decisions/domain-layer-not-flat-categories.md](decisions/domain-layer-not-flat-categories.md)。

对渲染的实际影响只有两处：首页的第一个网格是域卡片（不再是分类卡片），
工具页面包屑从域起（域 → 分类 → 条目）。路由多一条 `/domains/{id}`，见下表。

## 构建管线

四个 xtask 子命令，两个产物目录。**目录分工是硬要求，不是习惯**：

```
content/*.toml
   │  cargo build（build.rs）
   ▼
内嵌表 ──► xtask dump-html ──► dist/     全站 HTML（185 页 + 404.html）
                                 │
                                 │  npx pagefind --site dist --output-path public/pagefind
                                 ▼
                              public/    资源层：只有 Pagefind 索引 ← wrangler 上传这一个
                                 │
                                 └──（同一份索引复制回 dist/pagefind，让逃生舱也有搜索）
```

| 子命令 | 干什么 |
|--------|--------|
| `validate` | 内容五类校验，见 [reference/content-model.md](reference/content-model.md#五类校验) |
| `fetch` | 抓上游活跃度 → `content/generated/repo.json`（fail-soft） |
| `dump-html` | 走 Router 把 `all_paths()` 全渲一遍落盘到 `dist/` |
| `render-diff` | 「磁盘」与「构建期内嵌」两条内容路径逐字节比对 |
| `build-site` | `dump-html` + Pagefind 索引 → `public/` |

⚠️ **`dist/` 绝不能当资源目录。** Workers 是资源优先 —— 把全站 HTML 放进资源层，
每个页面都会被静态命中、Router 永远不被唤起，SSR 那半个架构就白搭了。
`public/` 只放构建期生成的客户端资源。

`dump-html` 与 `render-diff` 都通过 `site::router::render_path()` 走**真正的 Router**，
不直接调模板 —— 绕过 Router 去渲染就是又一份渲染路径，必然漂移。
`all_paths()` 与 `app()` 的一致性由单元测试钉住（清单里每条都必须返回 200）。

### render-diff 覆盖什么、不覆盖什么

✅ 覆盖：`build.rs` 生成的内嵌表是否忠实于 `content/`（页面清单 + 每页字节）。
这道门禁把「改了 TOML 但内嵌表没跟上」变成构建失败。

❌ **不覆盖：wasm32 运行时是否渲染出同一份字节。** 那需要在 CI 里用 wrangler 真起一个
Worker 再比，目前没有。[decisions/dual-target-axum.md](decisions/dual-target-axum.md)
承诺的「两个目标逐字节比对」因此只兑现了一半 —— 见
[ROADMAP.md](ROADMAP.md) 开放项 13。

## 请求路由：什么走静态资源，什么进 Worker

⚠️ 表里 `/calculator/*`、`/advisor/*`、`/api/*` 与 htmx.min.js **目前不存在**
（ADR calculator-advisor-justify-ssr，要等段 3 的 FieldNote 数据落库）。
其余各行已就位。

Workers 的默认顺序是**资源优先** —— 命中静态资源直接返回，压根不唤起 Worker，也就不计 CPU。
只有未命中才落到 Router。所以「SSR 的站」实际上大部分请求都是静态成本。

| 路径 | 由谁应答 | 说明 |
|------|---------|------|
| `/static/*`、字体、`htmx.min.js` | 资源层 | 不进 Worker，零 CPU |
| `/pagefind/*` | 资源层 | 构建期生成的搜索索引，纯客户端消费 |
| `/`、`/tools/{slug}`、`/domains/{d}`、`/categories/{c}`、`/replaces/{v}` | Worker · Maud SSR | 资源未命中 → 落到 Router |
| `/calculator/estimate`、`/advisor/rank` | Worker · HTMX 片段 | 返回 HTML 片段而非 JSON |
| `/api/tools.json`、`/index.xml`、`/tools.csv` | Worker | 机器可读的结构化导出 |

**部署走 Terraform**，且拆成两层：`cloudflare/terraform/modules/worker/` 是
**可复用子模块**（`cloudflare_worker` + `worker_version` + `workers_deployment`，
外加二选一的 `workers_custom_domain` / `workers_route` —— 本仓库配了前者，所以线上是 4 个；
provider ≥ 5.11 才有 `assets.directory`；不含 provider 与 backend），
`cloudflare/terraform/` 只是一个薄根模块。

**这个站点因此不只能由本仓库部署** —— homelab 或任何用 Terraform 管 Cloudflare 的项目
都可以 `source` 到那个子模块，用自己的账号、state 与 DNS 归属模型。本仓库只是它的
第一个消费者。消费方契约（关键一条：**构建产物由调用方负责生成**，Terraform 不编 Rust）见
[modules/worker/README.md](../cloudflare/terraform/modules/worker/README.md)。

`wrangler.jsonc` 保留在 `crates/edge/`，但只服务 `wrangler dev` 这类本地调试 ——
**不要用 `wrangler deploy`**：那样建出的 Worker 是 Terraform 不认识的资源，
之后第一次 apply 会因「已存在」而失败。
步骤见 [runbooks/deploy-cloudflare.md](runbooks/deploy-cloudflare.md)。

两处等价的关键配置（Terraform 侧在 `assets.config`，wrangler 侧在 `assets`）：

- ✅ `"not_found_handling": "none"` —— 未命中**不要**由资源层兜底，交给 Worker 渲染 404。
- ⬜ `"run_worker_first": [...]` —— 先没写。它防的是「同名静态资源遮蔽动态路由」，
  而那几条路由目前不存在，资源目录里也只有 `/pagefind/*`，遮蔽不了任何东西。
  等 calculator / advisor 落地时再加（配置文件里留了注释）。
- ✅ 自定义域名 `stack.meirong.dev`（2026-08-23）—— 走 Terraform 的 `custom_domain`
  变量（`cloudflare_workers_custom_domain`），**不在 `wrangler.jsonc` 里配 `routes`**：
  两处都写就是双主。`workers.dev` 子域仍开着做冒烟测试。
  归属取舍见 [runbooks/deploy-cloudflare.md](runbooks/deploy-cloudflare.md#自定义域名与-dns-归属冲突)。

## 技术栈

版本**以 `Cargo.lock` 的实际解析为准**，`cargo update` 之后回填本表。

| 组件 | 版本 | 状态 | 为什么是它 |
|------|------|:---:|-----------|
| `axum` | 0.8.9 | ✅ | Router 能被 workers-rs 直接 `call()`；⚠️ 0.8 起路径参数是 `{param}` 不是 `:param` |
| `worker` / `worker-macros` | 0.8.5 | ✅ | Cloudflare 官方 SDK，`axum` feature 是一等支持而非社区适配层 |
| `maud` | 0.27.0 | ✅ | 编译期检查的 HTML 宏。标签闭合、转义、类型全在 `cargo build` 抓 |
| `tower-service` | 0.3.3 | ✅ | 提供 `Service::call` —— Router 与 Workers 之间的胶水，也是 `render_path` 的底座 |
| `console_error_panic_hook` | 0.1.7 | ✅ | wasm 里 panic 默认只留一句 unreachable；装上它才有可读栈 |
| `serde` | 1.0.229 | ✅ | TOML 条目 → 强类型 `Catalog` |
| `tokio` | 1.53.1 | ✅ | 仅 `crates/dev`，**绝不进 edge 依赖树** |
| Pagefind | 1.5.2 | ✅ | 搜索。构建期索引 `dist/`，运行时纯客户端，**不消耗 Worker CPU**。走 `npx`，不往仓库塞 node 依赖 |
| htmx | 2.0.10 | ⬜ | 计算器 / Advisor 的交互。约 14 KB，作静态资源发，零构建步骤 |
| `pulldown-cmark` | 0.13.4 | ⬜ | 长文 Markdown → HTML，**只在构建期跑**。要等长文字段真的用上（ROADMAP 开放项 3） |

⬜ 那两行是**选型结论，不是现状** —— 还没进 `Cargo.toml`。

⚠️ `crates/edge` 的 `axum` 必须 `default-features = false` —— 不关默认 feature 会拖进
tokio/hyper，wasm32 编不过（完整上下文见
[decisions/dual-target-axum.md](decisions/dual-target-axum.md)）。

## 已知约束

| 约束 | 实际影响 | 触顶时怎么办 |
|------|---------|-------------|
| Worker 包体 ≤ 3 MiB gzip（Free） | ✅ **实测 346 KiB gz = 上限的 11%**（2026-08-22，`worker-build --release`）：`index_bg.wasm` 1.09 MB → 349 KiB gz，`index.js` 23 KB → 6 KiB gz。**已含**内嵌的 97 条条目（160 KiB 未压缩）。到几百条都宽裕 | 目录移入 Workers KV 或 D1，Router 一行不用改 |
| CPU 10 ms / 请求（Free） | Maud 渲染是微秒级，不构成压力 | **前提是 Markdown 在构建期就渲成 HTML**，绝不在 handler 里 parse |
| 依赖必须编到 `wasm32-unknown-unknown` | 没有 tokio net、没有 native TLS、**没有文件系统** | 运行时 I/O 已关进 `crates/xtask` 与 `crates/dev`；内容靠 `crates/site/build.rs` 在构建期内嵌（构建脚本不进产物）。这条约束不产生实际成本 |
| JS 支撑的 future 不是 `Send` | 只在 handler 内调 `worker::Fetch` 时才会咬人 | 运行时不发出站请求，天然规避 |
| `getrandom` 在 wasm32 需指定后端 | 一次性配置，几行 cfg | — |
| Rust → WASM 构建比 Hugo 慢 | 冷构建分钟级 | 日常开发只跑 `cargo run -p dev`（秒级），只有部署才编 wasm |

## 对 homelab 的影响

选 Cloudflare 而非 oracle-k3s 的理由（oracle 单向缩容至 2 OCPU/12GB + 六项运维负担）
不在这里复述，见
[decisions/cloudflare-workers-not-pages.md](decisions/cloudflare-workers-not-pages.md#为什么不是-oracle-k3s)。
结论是：这里的最优解是干脆不占容量。

| 这个站给 homelab 增加了什么 | |
|---|---|
| Pod / PVC / 那台 12 GB VM 上的内存 | 0 |
| PrometheusRule / Uptime Kuma monitor / Trivy 扫描目标 | 0 |
| restic 备份对象 | 0（内容即 git 仓库） |
| homelab 侧 Terraform 改动 | **0 条**（本仓库自己部署时）。⚠️ 但 homelab 也可以反过来**成为部署方** —— 它 `source` 到 `modules/worker` 自己部署这个站点。那种形态下改动落在 homelab 一侧，本仓库只提供模块与构建产物。DNS 归属的互斥选择（2026-08-23 定了方案 A）见 [runbooks/deploy-cloudflare.md](runbooks/deploy-cloudflare.md#自定义域名与-dns-归属冲突) |
| Homepage 磁贴 | 1 |

⚠️ 唯一的反向依赖是**构建期**从 homelab Prometheus 抓实测数据 —— 该步骤必须 fail-soft，
否则等于把公网站点的构建绑上了 homelab 的可用性。判据与实现见
[plans/2026-08-20-data-pipeline.md](plans/2026-08-20-data-pipeline.md)。

与 GitHub Pages 那两个主机名不同，**Workers 自定义域是走代理的** —— 不需要 DNS-only，
也就不绕过 WAF，且这个站只读无状态，不需要新增 rate limiting 规则
（homelab `ROADMAP` #11：Free 计划只允许一条，详情见
[decisions/cloudflare-workers-not-pages.md](decisions/cloudflare-workers-not-pages.md)）。
