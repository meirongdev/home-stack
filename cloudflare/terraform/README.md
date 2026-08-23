# cloudflare/terraform

两层：

| | 是什么 | 谁用 |
|---|--------|------|
| [`modules/worker/`](modules/worker/README.md) | **可复用子模块** —— 三个资源，无 provider、无 backend | 任何用 Terraform 管 Cloudflare 的项目 |
| 本目录（根） | 薄根模块：配 provider + backend，调用上面那个模块 | home-stack 自己部署自己 |

**home-stack 只是那个模块的第一个消费者。** homelab 或别的项目要部署这个站点，
不必复制这里的配置 —— 直接 `source` 到 `modules/worker`，用自己的账号、自己的 state、
自己的 DNS 归属模型。消费方契约（尤其是「构建产物由调用方负责生成」这条）见
[modules/worker/README.md](modules/worker/README.md)。

## 自己部署自己

```sh
export CLOUDFLARE_API_TOKEN=…      # Account: Workers Scripts:Edit + Account Settings:Read
export TF_VAR_account_id=…

just build      # 门禁 + Worker wasm + 资源层（Terraform 不编 Rust）
just init
just plan       # 应当是 Plan: 3 to add（配了域名则 4）
just apply
```

完整步骤、token 最小权限、验证清单与回滚见
[docs/runbooks/deploy-cloudflare.md](../../docs/runbooks/deploy-cloudflare.md)。
为什么落 Workers 而不是 Pages / oracle-k3s 见
[docs/decisions/cloudflare-workers-not-pages.md](../../docs/decisions/cloudflare-workers-not-pages.md)。

## 三个容易踩的地方

1. **`just build` 不是可选步骤，而且必须在 `plan` 之前。** Terraform 不编 Rust、
   不建 Pagefind 索引，而且 `plan` 阶段就会 `lstat` 资源目录 ——
   没先 build 的话 plan 直接失败。这是有意的：响亮地失败，好过静默部署一个空资源层。
2. **资源目录只能是 `public/`，绝不能是 `dist/`。** 后者是全站 HTML，
   进了资源层会让每个页面都被静态命中、Router 永远不被唤起。
3. **不要先用 `wrangler deploy` 试一把。** 那样建出的 Worker 是 Terraform 不认识的
   资源，之后第一次 `apply` 会因「已存在」失败，得先 `terraform import` 才能收回来。
   `wrangler` 在这个项目里只当调试工具（`wrangler dev`、`deployments list`）。

## 凭据与 state

- token 只从环境变量 `CLOUDFLARE_API_TOKEN` 读，**不作为 terraform 变量** ——
  变量会进 state，而 state 要被备份和传阅。
- **state 在 R2**（S3 兼容后端，key `home-stack/cloudflare.tfstate`，2026-08-23 迁入）。
  于是任何 terraform 命令都要两样东西：R2 的 S3 凭据（`AWS_ACCESS_KEY_ID` /
  `AWS_SECRET_ACCESS_KEY`）和 `-backend-config=backend.hcl`（endpoint 含账号 id，
  不进这个公开仓库 —— 用 `just init`，它带上了）。
  ☠️ 换成远端后端不是洁癖而是 CI 的硬前置：干净 runner + 本地 state = 每次从空开始，
  会去创建已存在的 Worker 然后报错。
- R2 凭据用**专用的窄 token**（Object Read & Write，范围只勾 `terraform-backend` 桶），
  不用「从宽权限 Cloudflare token 派生」那条路 —— 轮换任何一边会同时废掉另一边，
  而本仓库是公开的。见 [reference/cross-repo-boundary.md](../../docs/reference/cross-repo-boundary.md)。
- `.terraform.lock.hcl` **要提交**：它是「CI 与本机解析出同一个 provider」的唯一保证。
