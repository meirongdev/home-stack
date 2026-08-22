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
- 首次部署用本地 state 最省事。**CI 部署必须换成远端后端**：
  干净 runner + 本地 state = 每次从空开始，会去创建已存在的 Worker 然后报错。
  `versions.tf` 里有注释好的 R2（S3 兼容）配置块。
- `.terraform.lock.hcl` **要提交**：它是「CI 与本机解析出同一个 provider」的唯一保证。
