# 部署到 Cloudflare Workers

> 日期: 2026-08-23
> 状态: 🟢 步骤 1–5 **已完成**（2026-08-23 首次 apply，站点已公网可访问：Worker + 版本 +
> deployment 三个资源建成）；自定义域名那一个资源当次失败、修复已合入，**待实施**（见第 5 步）。
> 第 7 步（CI 部署）仍未验证 —— 它要求远端 state 后端，而本次用的是工作站本地 state

部署走 **Terraform**（`cloudflare/terraform/`），不走 `wrangler deploy`。
本篇是「home-stack 自己部署自己」；**别的项目要部署这个站点**看
[cloudflare/terraform/modules/worker/README.md](../../cloudflare/terraform/modules/worker/README.md)
——那是可复用子模块的消费方契约，本仓库只是它的第一个消费者。
选型理由不在这里（见
[decisions/cloudflare-workers-not-pages.md](../decisions/cloudflare-workers-not-pages.md)），
架构形态也不在这里（见 [ARCHITECTURE.md](../ARCHITECTURE.md#构建管线)）。

> ☠️ **不要先用 `wrangler deploy` 试一把。** 那样 Worker 会以「Terraform 不知道的资源」
> 形式存在，之后第一次 `terraform apply` 会因为「已存在」而失败，得先
> `terraform import` 才能收回来。**从第一次部署就走 Terraform。**
> `wrangler` 在这个项目里只当调试工具用（`wrangler dev`、`deployments list`）。

## 先决条件

| 需要什么 | 说明 |
|---------|------|
| Cloudflare 账号 | Free 计划够用。实测包体只占上限的 11%（见第 3 步） |
| `terraform` ≥ 1.9 | ✅ 本机 1.15.5 验证过 |
| `just` | ✅ 本机 1.58.0。不装也行，照着 `justfile` 里的命令手敲 |
| Node（含 `npx`） | 只用来跑 `pagefind`，仓库里没有 node 依赖 |
| `rustup target add wasm32-unknown-unknown` | 已装则跳过 |
| `cargo install worker-build` | ✅ 本机 0.8.5 验证过。它会自行下载 `wasm-bindgen` 与 `wasm-opt` |

⚠️ **域名可以先不定。** 不配 `custom_domain` 就只用 `workers.dev` 子域，
足够兑现段 2 的出口判据「公网可访问」，而且不碰任何 DNS。
配自定义域名会牵出一个归属冲突，见[最后一节](#自定义域名与-dns-归属冲突)。

## 1. 建 API token（Cloudflare 控制台）

Dashboard → 右上头像 → **API Tokens** → Create Token → Custom token。
**最小权限**：

| 作用域 | 权限 | 干什么用 |
|--------|------|---------|
| Account | Workers Scripts : **Edit** | 建 Worker、上传版本与静态资源 |
| Account | Account Settings : **Read** | provider 读账号信息 |
| Zone（仅配自定义域名时） | Workers Routes : **Edit** | 绑自定义域 |

Resources 只勾**这一个账号**（和那一个 zone），不要 All accounts。

```sh
export CLOUDFLARE_API_TOKEN=…
export TF_VAR_account_id=…        # Dashboard 右侧栏可复制
```

⚠️ token 只走环境变量，**不要**写进 `.tfvars` 或 terraform 变量 ——
那会让它进 state 文件，而 state 是要被备份和传阅的。

## 2. 构建（仓库根 / `cloudflare/terraform/`）

```sh
cd cloudflare/terraform
just build
```

这一步等价于四条命令：`xtask validate` → `xtask render-diff` →
`xtask build-site` → `worker-build --release`。

☠️ **`just build` 不是可选步骤，而且必须在 `plan` 之前。**
Terraform 不编 Rust、不建 Pagefind 索引；而且它在 **plan 阶段就会 `lstat` 资源目录** ——
没先 build 的话 plan 直接报 `lstat …/public: no such file or directory`。
这是有意的失败：响亮地失败，好过静默部署一个空资源层。

产出两个目录，**分工是硬要求**：

- `public/` —— 资源层，Terraform 只上传这个。约 1.0 MB，全是 Pagefind 索引。
- `dist/` —— 全站 HTML（185 页 + `404.html`），纯静态逃生舱用。

☠️ **把 `dist/` 当资源目录是这套部署里最严重的一种配错。** Workers 是资源优先：
全站 HTML 进了资源层，每个页面都被静态命中、Router 永远不被唤起 ——
站点看起来正常，但 SSR 那半个架构完全失效。变量默认值已经写死成 `../../public`，别改。

## 3. 包体实测（可选，但第一次值得看一眼）

✅ 已验证（2026-08-22，worker-build 0.8.5）：

| 文件 | 原始 | gzip |
|------|------|------|
| `build/index_bg.wasm` | 1,092,552 B | 348,668 B |
| `build/index.js` | 22,780 B | 5,960 B |
| **合计** | | **≈ 346 KiB = Free 上限 3 MiB 的 11%** |

**已包含**内嵌的 97 条条目（160 KiB 未压缩）—— 内容进包体不构成压力。

## 4. plan（`cloudflare/terraform/`）

```sh
just init
just plan
```

✅ 已验证：provider 5.23.0 下 `terraform validate` 与 `terraform plan` 全绿，
输出应当是 **`Plan: 3 to add, 0 to change, 0 to destroy`**（Worker、Version、Deployment）。
配了 `custom_domain` 则是 4 个。

读一遍再往下。特别看：`modules` 是不是两个（`index.js` + `index_bg.wasm`）、
`assets.directory` 指的是不是 `public`。

## 5. apply

> ✅ **2026-08-23 首次执行**（工作站，本地 state，带 `custom_domain` 的 4 资源 plan）。
> 前 3 个成功：Worker（1s）→ 版本（上传两个 module + `public/` 里 114 个文件，14s）→
> deployment 100%。站点当场公网可访问。
>
> ☠️ **第 4 个 `cloudflare_workers_custom_domain` 失败**：
> `400 / 100124 Cannot attach custom domain: Worker 'home-stack' has no deployments`。
> 不是权限也不是配置错，是**依赖图缺一条边** —— 那个资源只引用 worker 的 `name`，
> Terraform 于是把它和「上传资源层要十几秒」的版本资源并发执行，撞上「Worker 还没有
> deployment」。已在 `modules/worker/main.tf` 补 `depends_on`（那里有完整注释），
> 修完重跑即可：已建成的 3 个资源不受影响，plan 只剩 `1 to add`。
>
> ⚠️ **部分失败时退出码是 1，但 state 已经写了**（3 个资源在里面）。别把它当成
> 「什么都没发生」而去重建 —— 那会撞上本文开头那条「Worker 已存在」的坑。

```sh
just apply
```

Terraform 会：建 Worker → 上传两个 module + 扫 `public/` 分片上传资源
→ 建版本 → 把 100% 流量指向它。

完成后从 Dashboard 拿到 `https://home-stack.<你的子域>.workers.dev`，逐条核对：

```sh
BASE=https://home-stack.<你的子域>.workers.dev
curl -sI  $BASE/ | head -1                                      # 200
curl -s   $BASE/tools/cilium | grep -o '<title>[^<]*'           # 条目页
curl -s   $BASE/domains/networking | grep -o 'class="card"' | wc -l  # 应为 12
curl -sI  $BASE/pagefind/pagefind-ui.js | head -1               # 200 ← 资源层通了
curl -s   $BASE/nope/nope | grep -o '没有这个页面'                # Worker 渲染的 404
curl -s -o /dev/null -w '%{http_code}\n' $BASE/nope/nope        # 应为 404
```

⚠️ **上面两条命令都是 2026-08-23 修过/补过的,原因值得记住**：

- 卡片数那条原先写的是 `grep -c '<a class="card"'` —— 它**永远返回 1**。
  线上 HTML 是压成一行的，`grep -c` 数的是**行数**不是出现次数。
  一条恒真的验收命令比没有验收更糟。
- 404 那条原先只 grep 页面文字，**不看状态码**。实测未知路径返回的是
  **HTTP 200**（soft 404）——页面对、状态码错，爬虫会把无限多不存在的 URL
  当正常页面收录。⏸ 这是 `crates/edge` 侧的待修缺陷，不是部署配置问题。

☠️ **两个「站点看起来正常但其实坏了」的症状,第一次部署务必核对**：

1. `/pagefind/pagefind-ui.js` 非 200 → 资源层没上传成功。站点完全正常，
   只是搜索框不出现，**没有任何报错**。
2. 页面能开但改了模板重新部署后不生效 → 资源目录配成了 `dist/`（见第 2 步）。

## 回滚

内容或代码出错时**首选 git**：真相源在仓库里，不在 Cloudflare。

```sh
git revert <引入问题的 commit>
cd cloudflare/terraform && just build && just apply
```

`just rollback-help` 会打印这套步骤，以及「临时指定历史版本」的做法。
写错一条条目其实压根到不了线上 —— `just build` 里的 `validate` 会先把它挡下来。

## 6. 别的项目怎么部署它

`cloudflare/terraform/` 是两层：`modules/worker/` 是**不含 provider、不含 backend**
的可复用子模块，本目录只是一个薄根模块把自己仓库的构建产物路径喂进去。
homelab 或任何用 Terraform 管 Cloudflare 的项目照同样方式调用它即可：

```hcl
module "home_stack" {
  source = "github.com/meirongdev/home-stack//cloudflare/terraform/modules/worker?ref=<tag>"

  account_id       = var.cloudflare_account_id
  name             = "home-stack"
  worker_build_dir = "${path.root}/.build/home-stack/crates/edge/build"
  assets_dir       = "${path.root}/.build/home-stack/public"

  # DNS 归属：zone 已由自己的 Terraform 全量管理时用 route，模块不碰 DNS
  route = { pattern = "stack.example.dev/*", zone_id = var.zone_id }
}
```

☠️ **消费方必须自己构建。** 远程 `source` 只给 `.tf` 文件 —— wasm 与资源层不在里面，
也不可能在里面（1 MB 的二进制产物，随每次内容改动而变）。CI 里的做法是
再 checkout 一次 home-stack、跑那四条构建命令，再 apply。完整片段与工具链清单在
[模块 README](../../cloudflare/terraform/modules/worker/README.md#契约模块不构建调用方构建)。

⚠️ `ref` 要钉死在 tag 上，别用 `main` —— 内容与代码一起变，钉 `main` 意味着
你的部署内容会在你没改任何东西的时候变。⏸ 这个仓库目前**还没有任何 tag**
（ROADMAP 开放项 17）。

## 7. 交给 CI

[`.github/workflows/deploy.yml`](../../.github/workflows/deploy.yml) 已经写好整条链
（门禁 → `build-site` → `worker-build` → `terraform plan` → `apply` 那份 plan 文件），
但它**只能手动触发**，且从未成功跑过。

☠️ **CI 部署有一个硬前置：远端 state 后端。**
Terraform 的 state 是「线上现在是什么」的唯一记录。CI 每次都是干净 runner，
用本地 state 等于每次都从空开始 —— 它会试图创建已经存在的 Worker 然后报错。
`versions.tf` 里有注释好的 R2（S3 兼容）后端配置块，与 homelab 仓库的做法一致。
**没配后端就不要用 CI 部署**，从工作站部署（那里 state 是持久的）。
workflow 第一步就 fail-closed 地检查这件事。

需要的 secret：`CLOUDFLARE_API_TOKEN`、`CLOUDFLARE_ACCOUNT_ID`，
以及 R2 后端的 `R2_ACCESS_KEY_ID` / `R2_SECRET_ACCESS_KEY`。

## 自定义域名与 DNS 归属冲突

☠️ **这里有一个必须先决定的冲突，不是配置细节。**

Workers 的 Custom Domain 会**自己创建 DNS 记录并签证书**，而且
**不能建在已存在 CNAME 记录的主机名上**（Cloudflare 文档明说）。
而 homelab 仓库的 `cloudflare/terraform` 管着 `meirong.dev` 这个 zone 的 DNS。
**两者互斥**，必须选一个所有者：

| 方案 | 怎么配 | 代价 |
|------|--------|------|
| **A. 这个仓库拥有** | 设 `custom_domain` 与 `zone_name` 变量（会创建 `cloudflare_workers_custom_domain`）。homelab 那边**不要**声明这条记录 | 那个 zone 的 DNS 不再只有一个真相源。⚠️ 若 homelab 侧有 prune 逻辑，它会试图删掉这条「不在它代码里」的记录 |
| **B. homelab 拥有** | 这里留空 `custom_domain`；homelab 建一条代理开启的记录，再在这里换成 `cloudflare_workers_route`（按 zone + pattern 绑，不碰 DNS） | 域名与 Worker 路由分别在两个仓库声明，改名要同时改两处 |

⚠️ **两边都要留注释。** 这是又一条跨仓库依赖，和 [ROADMAP.md](../ROADMAP.md)
开放项 5（Tailscale ACL）同一类：日后有人在另一个仓库里「清理」它，
症状是站点域名突然不解析，而这个仓库里没有任何线索指向原因。

定下来之前**先用 `workers.dev`** —— 它不碰任何 DNS，「公网可访问」照样兑现。
