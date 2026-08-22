# modules/worker — 把 Home Stack 部署到 Cloudflare Workers

可复用 Terraform 子模块。**home-stack 仓库自己只是它的第一个消费者** ——
homelab 或任何用 Terraform 管 Cloudflare 的项目都可以调用它，把这个站点
部署进自己的账号、自己的 state、自己的 DNS 归属模型里。

```hcl
module "home_stack" {
  source = "github.com/meirongdev/home-stack//cloudflare/terraform/modules/worker?ref=<tag>"

  account_id = var.cloudflare_account_id
  name       = "home-stack"

  # ☠️ 由**你**负责生成，见下节
  worker_build_dir = "${path.root}/.build/home-stack/crates/edge/build"
  assets_dir       = "${path.root}/.build/home-stack/public"
}
```

## 契约：模块不构建，调用方构建

☠️ **这是使用本模块最容易踩的一脚。** Terraform 不编 Rust、不建 Pagefind 索引。
远程 `source` 只会给你一份 `.tf` 文件 —— wasm 与资源层**不在里面**，
也不可能在里面（那是 1 MB 的二进制产物，随每次内容改动而变）。

所以调用方必须先 checkout home-stack 并构建：

```sh
git clone --depth 1 --branch <tag> https://github.com/meirongdev/home-stack .build/home-stack
cd .build/home-stack
cargo run -p xtask -- validate      # 内容五类校验
cargo run -p xtask -- render-diff   # 两条内容路径逐字节一致
cargo run -p xtask -- build-site    # → public/（资源层）与 dist/（纯静态逃生舱）
cd crates/edge && worker-build --release   # → build/index.js + build/index_bg.wasm
```

需要的工具链：Rust（含 `wasm32-unknown-unknown` target）、`cargo install worker-build`、
Node（`npx`，只为跑 pagefind）。

⚠️ **`plan` 阶段就会 `lstat` 这两个目录** —— 没构建就 plan 会直接失败，
而不是等到 apply。这是有意的：响亮地失败，好过静默部署一个空资源层。

### 在 CI 里的样子（以 homelab 这类项目为例）

```yaml
- uses: actions/checkout@v4                    # 你自己的仓库
- uses: actions/checkout@v4                    # 再 checkout home-stack
  with:
    repository: meirongdev/home-stack
    ref: <tag>
    path: .build/home-stack
- uses: dtolnay/rust-toolchain@stable
  with: { targets: wasm32-unknown-unknown }
- uses: actions/setup-node@v4
  with: { node-version: "22" }
- run: |
    cd .build/home-stack
    cargo run -p xtask -- validate
    cargo run -p xtask -- build-site
    cargo install -q worker-build
    cd crates/edge && worker-build --release
- run: terraform apply -auto-approve           # 你自己的根模块
```

⚠️ **`ref` 要钉死在一个 tag 上，别用 `main`。** 内容与代码是一起变的：
钉 `main` 意味着你的部署内容会在你没改任何东西的时候变。

## 输入

| 变量 | 必填 | 说明 |
|------|:---:|------|
| `account_id` | ✅ | Cloudflare 账号 ID |
| `name` | ✅ | Worker 名字。**没有默认值是故意的** —— 名字在账号内唯一，给默认值会让同账号第二次部署静默覆盖第一次 |
| `worker_build_dir` | ✅ | 含 `index.js` 与 `index_bg.wasm` 的目录。绝对路径，或相对于**调用方**工作目录 |
| `assets_dir` | ✅ | 资源层目录（`public/`）。☠️ **绝不能给 `dist/`** —— 那是全站 HTML，进资源层会让每个页面都被静态命中、Router 永不被唤起 |
| `compatibility_date` | | 默认跟 home-stack 实测过的日期。它是**代码的属性**，所以默认值在模块里而不是留给你猜 |
| `workers_dev_enabled` | | 默认 `true` |
| `observability_enabled` | | 默认 `true` |
| `custom_domain` | | 方案 A（见下） |
| `route` | | 方案 B（见下） |

路径不做 `path.module` 拼接：远程 source 消费时 `path.module` 指向 provider 的下载缓存目录，
而产物不在那儿。

## 两种 DNS 归属，必须选一个

两个都留空 = 只用 `workers.dev`，完全不碰 DNS。适合先验证「公网可访问」。

| | 怎么配 | 谁拥有 DNS | 什么时候选它 |
|---|--------|-----------|------------|
| **A** | `custom_domain = { hostname, zone_name }` | **Cloudflare** 自己建记录并签证书 | 那个 zone 的 DNS 不由 Terraform 全量管理 |
| **B** | `route = { pattern, zone_id }` | **调用方** —— 你自己建一条代理开启的记录，模块只绑路由 | 你的 Terraform 已经全量管理该 zone，不希望有记录游离在代码外 |

同时设两个会被 precondition 拦下（已实测）。

☠️ 选 A 时注意：Custom Domain **不能建在已存在 CNAME 记录的主机名上**，
而且你的 Terraform 里**不要**再声明同一条记录 —— 两份 state 会打架，
带 prune 的一侧会试图删掉「不在它代码里」的那条。
⚠️ 无论选哪个，**两边都要留注释**：日后有人清理另一侧，症状是站点域名突然不解析，
而这一侧没有任何线索指向原因。

## 输出

`worker_name` / `worker_id` / `version_id` / `workers_dev_hostname` / `url`。

⚠️ `workers_dev_hostname` 只是主机名前半段 —— 账号子域是账号级设置，provider 不返回。

## 内部约定（改模块前要知道）

- 三个资源对应 Cloudflare 的三层模型：**Worker → Version → Deployment**。
  版本不可变，部署只是「指向哪个版本」—— 这也正是它能被声明式管理的原因。
- 上传的是**两个 module**：`index.js`（`application/javascript+module`）与
  `index_bg.wasm`（`application/wasm`）。后者的 `name` 必须是 `index_bg.wasm`，
  因为 `index.js` 里写着 `import X from "./index_bg.wasm"`。
  ☠️ 写错的症状很难看：`terraform apply` 是绿的，Worker 启动即 500。
- 本模块**不含 `provider` 块也不含 backend** —— 那是根模块的职责。
