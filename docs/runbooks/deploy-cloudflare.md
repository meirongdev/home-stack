# 部署到 Cloudflare Workers

> 日期: 2026-08-23
> 状态: 🟢 步骤 1–5 **已完成**（2026-08-23 首次 apply，站点已公网可访问：Worker + 版本 +
> deployment 三个资源建成）；自定义域名那一个资源当次失败、修复已合入，**待实施**（见第 5 步）。
> 第 7 步 ✅ **全部完成**（2026-08-23）：state 已迁入 R2，`deploy.yml` 在干净 runner 上
> 首跑通过（run 32645852445），站点实测正常

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

✅ **域名已定：`stack.meirong.dev`**（2026-08-23，方案 A）。它写在
`cloudflare/terraform/terraform.tfvars` 的 `custom_domain` 里，归属取舍见
[最后一节](#自定义域名与-dns-归属冲突)。
⚠️ 从零复现时仍可**先不配** `custom_domain` —— 只用 `workers.dev` 子域就能兑现
「公网可访问」，而且不碰任何 DNS；等归属定了再补那一个资源（这也正是本次的实际路径）。

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

⚠️ **state 在 R2**（2026-08-23 迁入），所以 `just init` 要带上 R2 的 S3 凭据与
`backend.hcl` —— 见第 7 步。少任何一样都会在「缺 endpoint」或「凭据无效」处响亮失败，
不会静默退回本地文件。

✅ 已验证：provider 5.23.0 下 `terraform validate` 与 `terraform plan` 全绿。
⚠️ **plan 长什么样取决于是不是首次部署**：
- 首次（空 state）：**`Plan: 3 to add, 0 to change, 0 to destroy`**（Worker、Version、
  Deployment），配了 `custom_domain` 则是 4 个
- 站点已在线、改了内容或代码后重新部署：**`2 to add, 2 to destroy`** —— 新版本 +
  deployment 重建，Worker 与 custom domain 不动、证书不重签（2026-08-23 实测）
- 什么都没改、也没重 build：**`No changes`**

☠️ **CI 上永远不会是 `No changes`**，别拿它当判据。干净 runner 编出的 wasm 与 Pagefind
索引跟工作站那批不是同一份字节 —— 2026-08-23 实测 `asset_manifest_sha256`
`3cc06409…` → `7e8cfd64…`，连同两个 module 条目一起 forces replacement。
CI 上「state 真接上了」的判据是另一条，见第 7 步。

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
  source = "github.com/meirongdev/home-stack//cloudflare/terraform/modules/worker?ref=v0.1.0"

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
你的部署内容会在你没改任何东西的时候变。✅ 首个可钉版本是 **`v0.1.0`**（2026-08-23，
指向首次 apply 成功、站点实测可访问的那个状态）；当前有哪些 tag：
`git ls-remote --tags origin`。⚠️ 消费方那边还有一处 `ref`（checkout home-stack 来构建），
两处必须是同一个 tag —— 见[模块 README](../../cloudflare/terraform/modules/worker/README.md)。

## 7. 交给 CI

[`.github/workflows/deploy.yml`](../../.github/workflows/deploy.yml) 已经写好整条链
（门禁 → `build-site` → `worker-build` → 写 `backend.hcl` → `terraform plan` →
`apply` 那份 plan 文件），**只能手动触发**。

☠️ **CI 部署的硬前置是远端 state 后端。** Terraform 的 state 是「线上现在是什么」的
唯一记录。CI 每次都是干净 runner，本地 state 等于每次从空开始 —— 它会试图创建已经
存在的 Worker 然后报错。

### ✅ 远端 state：2026-08-23 已迁入 R2

key 是 `terraform-backend/home-stack/cloudflare.tfstate`。⚠️ **桶本体不归本仓库** ——
只拥有这个 key 前缀，见 [reference/cross-repo-boundary.md](../reference/cross-repo-boundary.md)。

下面四步当时是**一起**做完的，别只做一半 —— 两种半成品的症状都很难查：
只取消注释（不迁移）→ 本地 `plan`/`apply` 卡在「要 init 到 R2」，而 CI 那道检查会放行
→ CI 拿到空 state，去创建已存在的 Worker；只迁移（不取消注释）→ terraform 继续读本地
文件，R2 那份从此过期，而**两边都「看着正常」**。

```sh
# 1) R2 的 S3 凭据：Dashboard → R2 → Manage R2 API tokens → Create API token，
#    权限 Object Read & Write、范围只勾 terraform-backend 桶。
#    ⚠️ 别走「从已有 Cloudflare token 派生」那条（AK = token id、SK = token 值的 SHA-256）：
#    它省的只是一次 Dashboard 往返 —— 而 CI 那枚窄 token 无论如何都得建，所以什么也没省，
#    代价却是轮换任何一边会同时废掉另一边，而本仓库是公开的。
export AWS_ACCESS_KEY_ID=…
export AWS_SECRET_ACCESS_KEY=…

# 2) endpoint（含账号 id，不进这个公开仓库）
cp backend.hcl.example backend.hcl && $EDITOR backend.hcl

# 3) 取消 versions.tf 里 backend 块的注释

# 4) 迁移。terraform 问一次「copy existing state to the new backend?」——要。
just migrate-state
```

⚠️ **迁移前先确认手上那份备份是最新的 —— 比 serial，别看文件名。** 2026-08-23 踩到过：
留着的那份 `terraform.tfstate.pre-r2-backup` 是 **serial 7**，指向 404 修复**之前**的
worker version，而线上真相是 **serial 12**。拿它还原会让 terraform 以为线上跑的是旧版本。

```sh
python3 -c 'import json;d=json.load(open("terraform.tfstate"));print(d["serial"],len(d["resources"]))'
```

迁完的**实测长相**（旧直觉在这里会骗人）：

| 看哪里 | 实测 |
|---|---|
| `.terraform/terraform.tfstate` | `backend.type = s3`，bucket / key / `region = auto` 与四个 `skip_*` 都记着 |
| 本地 `terraform.tfstate` | **被清空成 0 字节** —— 不是「留在原地」。看到它别以为 state 丢了 |
| `terraform.tfstate.backup` | 迁移前那份本地内容落在这里（serial 12） |
| 查现状 | `terraform state list`，读的是 R2 |
| R2 里那份 state | `serial 1` + **一条全新的 lineage** |

☠️ **别拿 serial / lineage 去判断迁移成不成功。** 直觉是「拷过去应该原样保留」，实际
terraform 往一个**空**的目标后端持久化时会重新签发两者：serial 从 0 起步（于是是 1），
lineage 另起一条。2026-08-23 实测：本地是 `serial 12 / 081799fc`，迁完 R2 里是
`serial 1 / 0974a83f` —— 而 4 个资源地址一个不少。**看资源，不看这两个数**：

```sh
terraform state list   # 要 4 行：worker / worker_version / workers_custom_domain[0] / workers_deployment
```

📌 那条 LibreSSL 的担心不成立：homelab 把 R2 后端注释掉时归因于「本机握手失败」，
但 terraform 是静态链接的 Go、自带 TLS 栈 —— 同一台机器上迁移一次通过。
真在本地迁不动了再走绕法（从 runner 迁，或用 rclone / `npx wrangler r2 object put`
把 `terraform.tfstate` 直接推到那个 key，再 `just init`）。

### ✅ CI 部署：2026-08-23 首跑通过

四个 secret（`gh secret set <名字> --repo meirongdev/home-stack` 交互式收值，
不进 shell history）：`CLOUDFLARE_ACCOUNT_ID`、`R2_ACCESS_KEY_ID`、
`R2_SECRET_ACCESS_KEY`、`CLOUDFLARE_API_TOKEN`。

☠️ 最后那枚必须是**新建的窄 token**（Workers Scripts: Edit + Account Settings: Read +
zone Workers Routes: Edit），**不是** homelab 那枚能改全 zone DNS / 隧道 / WAF 的 ——
本仓库是公开的。R2 那对同理，只给 `terraform-backend` 一个桶。
✅ 实测这三条权限**够了**：plan 能 refresh 四个资源、apply 能建版本与 deployment。

首跑：[run 32645852445](https://github.com/meirongdev/home-stack/actions/runs/32645852445)，
3 分 37 秒，`Apply complete! Resources: 2 added, 0 changed, 2 destroyed`。

☠️ **验收不是「workflow 绿了」。** 第一步那道 fail-closed 检查只 grep「配置里有没有
backend 块」，看不出 state 是否真接上 —— 空 state 上重建也会一路绿到 apply 才炸。
判据是 `terraform plan` 那步**碰了哪些资源**：

| 看什么 | 接上了的样子（2026-08-23 实测） | 空 state 的样子 |
|---|---|---|
| `cloudflare_worker.this` | 只 `Refreshing state... [id=5db1b949…]`，不在变更里 | `will be created` |
| `workers_custom_domain.this[0]` | 只 `Refreshing state... [id=558892dd…]` | `will be created`（然后撞「已存在」）|
| 汇总 | `2 to add / 2 to destroy` | `4 to add` ← **看到这个立刻停** |

📌 而且日志顺手给出了一条更硬的证据：apply 销毁的旧版本 id 是
`56bbb3c1-1dcc-4078-a767-5f66a20245fb` —— 正是工作站那次部署留下的版本。
说明 R2 里那份 state 不只是「有四个资源」，内容也是**最新**的那一份。

```sh
gh run watch <run-id> --repo meirongdev/home-stack --exit-status
gh run view <run-id> --repo meirongdev/home-stack --log | grep -E 'Refreshing state|must be replaced|Apply complete'
```

⚠️ **每次 apply 都有一段「Worker 没有任何 deployment」的窗口。** 实测顺序是
**先销毁 deployment（0s）→ 建新版本（5s）→ 建新 deployment（2s）**，窗口 ≈ **7 秒**
（`create_before_destroy` 只加在 version 上，deployment 没有）。那正是首次部署撞
`400 / 100124 Worker has no deployments` 的同一状态。⏸ **未决**：要不要给 deployment
也加 `create_before_destroy` —— 同一个 Worker 能不能短暂存在两个 deployment 没验过。

✅ 首跑后站点实测（CI apply 完约 2 分钟）：`/`、`/tools/prometheus`、
`/domains/observability`、`/pagefind/pagefind-ui.js` 全 200，随机 miss 路径 404，
首页标题与 97 条计数都在。⚠️ apply 完**立刻** curl 会拿到假阴性 —— 版本传播要十几秒。

## 自定义域名与 DNS 归属冲突

☠️ **这里有一个必须先决定的冲突，不是配置细节。**

Workers 的 Custom Domain 会**自己创建 DNS 记录并签证书**，而且
**不能建在已存在 CNAME 记录的主机名上**（Cloudflare 文档明说）。
而 homelab 仓库的 `cloudflare/terraform` 管着 `meirong.dev` 这个 zone 的 DNS。
**两者互斥**，必须选一个所有者：

| 方案 | 怎么配 | 代价 |
|------|--------|------|
| **A. 这个仓库拥有** ← **2026-08-23 选了这个** | 设 `custom_domain` 与 `zone_name` 变量（会创建 `cloudflare_workers_custom_domain`）。homelab 那边**不要**声明这条记录 | 那个 zone 的 DNS 不再只有一个真相源 |
| **B. homelab 拥有** | 这里留空 `custom_domain`；homelab 建一条代理开启的记录，再在这里换成 `cloudflare_workers_route`（按 zone + pattern 绑，不碰 DNS） | 域名与 Worker 路由分别在两个仓库声明，改名要同时改两处 |

**为什么是 A**（原先这张表担心「homelab 侧的 prune 逻辑会删掉这条记录」，2026-08-23 实测
那个担心不成立，于是 A 的代价只剩「真相源不唯一」这一条）：

- homelab 的两个 external-dns 实例都是 `policy: upsert-only` —— 只增改、**从不删**；
- 它的 `cloudflare_dns_record.subdomains` 是一个**空 map** 的 `for_each`；
- terraform 本身不会 prune 不在自己 state 里的记录。

也就是说自动化不会碰这条记录，**唯一的风险是人**。

⚠️ **两边都要留注释,这不是形式。** 这是又一条跨仓库依赖，和 [ROADMAP.md](../ROADMAP.md)
开放项 5（Tailscale ACL）同一类：日后有人在另一个仓库里「清理」它，
症状是站点域名突然不解析，而这个仓库里没有任何线索指向原因。
✅ homelab 侧的注释已经落在三处（2026-08-23）：`cloudflare/terraform/main.tf` 里
`cloudflare_dns_record.external_origins` 上方那段、`docs/reference/networking-ingress.md`
的「不走这条链的 meirong.dev 主机名」表、以及服务清单 `docs/reference/services.md`。

📌 记录长相是 `AAAA stack.meirong.dev → 100::`（橙云）。`100::` 是 IPv6 丢弃地址段，
Workers 自定义域名建的就是这种占位记录 —— 真实流量在边缘被截走。**别当成配错去"修"。**
另外这个主机名走橙云,于是吃 `meirong.dev` 的 zone 级 WAF 与限流。
