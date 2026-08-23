# 跨仓库边界：这个仓库拥有什么，不拥有什么

> 日期: 2026-08-23
> 状态: ✅ 已完成（分工已实施，两边都留了注释）

`stack.meirong.dev` 跑在 Cloudflare Workers 上，但那个域名所在的 zone
（`meirong.dev`）与那个 Cloudflare 账号是**另一个仓库**在管 —— 作者的基础设施仓库
（homelab，双集群 k3s 的 IaC，**私有**）。同一个账号里因此有两个仓库、两份
Terraform state、两枚凭据在写。

⚠️ **本篇必须自成一体**：那个仓库是私有的，读者点不进去。所以这里只写「本仓库拥有
什么、不许碰什么」，不把对面的内部结构搬过来。

## 本仓库拥有

| 资源 | 声明在哪 |
|------|---------|
| Worker 本体、不可变版本、deployment（哪个版本吃 100% 流量） | `cloudflare/terraform/modules/worker` |
| 静态资源层（Pagefind 索引） | 同上（`assets.directory`）|
| `stack.meirong.dev` 这**一条** DNS 记录 | `cloudflare_workers_custom_domain`（Cloudflare 自建 `AAAA 100::`，橙云）|
| 远端 state 里 `home-stack/` 这个 key 前缀 | `versions.tf` 的 backend 块（✅ 2026-08-23 启用，`terraform-backend/home-stack/cloudflare.tfstate`）。⚠️ **桶本体不归本仓库** —— 只拥有这个 key 前缀 |
| 应用层正确性：内容校验、两条渲染路径一致、包体大小 | `xtask validate` / `render-diff` / CI 九道门禁 |

## 本仓库不许碰

- **zone 的其余 DNS 记录**。那边有自动化在管（按 ownership 标记各管一摊），
  本仓库只碰自己那一条。
- **zone 设置 / WAF / 限流 / 隧道配置**。它们是**全 zone 共享**的，几十个主机名依赖同一份。
- **R2 桶本体与生命周期**。桶是那边建的，这里只拥有自己的 key 前缀。
- ☠️ **别在这一条记录上再叠一层归属**：Workers 自定义域名**不能**建在已存在 CNAME 的
  主机名上。两边都声明必然打架 —— 谁先 apply 谁赢，另一边永久报错。
  同理 `wrangler.jsonc` 里**刻意不写 `routes`**（那会与 Terraform 的
  `custom_domain` 争同一个归属，见 [ARCHITECTURE.md](../ARCHITECTURE.md#请求路由什么走静态资源什么进-worker)）。

## 依赖但不控制的事实

- ⚠️ **这个 host 走橙云，于是受 zone 级 WAF 与限流约束** —— 那些规则不在本仓库，
  而且免费档的规则位已经用满。所以：**不要把站点设计成需要 WAF 例外的样子**
  （比如别指望给爬虫或某条路径单独开口子）。真需要例外，得去基础设施那边提，
  并且很可能要先砍掉一条现有规则。
- ⚠️ **可用性监控在那边**（Uptime Kuma + 导航页磁贴）。本仓库**不自建**探活 ——
  一个站点自己监控自己没有意义。本仓库负责的是「内容与渲染是否正确」，
  不是「它是否还活着」。
- **凭据各自最小化**：本仓库的部署 token 只要 Workers Scripts: Edit +
  Account Settings: Read（配自定义域名再加 Zone Workers Routes: Edit）。
  ☠️ **绝不要**把基础设施仓库那枚能改全部 DNS/隧道/WAF 的宽 token 拿来用 ——
  尤其**本仓库是公开的**，它的 Actions secret 不该持有那种权限。

## 改名要动两边

换主机名不是改一个变量：本仓库改 `cloudflare/terraform/terraform.tfvars` 的
`custom_domain`，那边要同步改三处注释/清单（DNS 归属注释、入口链路文档、服务清单）。
⚠️ 跨仓库依赖的通病是「另一边有人清理它，而这一边没有任何线索」——
所以两边都留了注释，删注释等于拆掉唯一的护栏。

对面那份对称文档是一份 ADR（`decisions/home-stack-repo-boundary.md`，含完整归属表与
被否决的两个方案）。**两份都改才叫改完。**
