# Runbooks

> 日期: 2026-08-22

**照着做就能完成的操作步骤**。和其他三类的分工：
[decisions/](../decisions/README.md) 说为什么（明确**不收**步骤）、
[plans/](../plans/README.md) 是冻结的规格档案、
[reference/](../reference/README.md) 是「现在是什么样」的事实。
步骤没有第四个去处，所以有了这一类。

| Runbook | 什么时候读 |
|---------|-----------|
| [deploy-cloudflare.md](deploy-cloudflare.md) | 第一次把站点部署到 Cloudflare Workers，或事后回滚 |

## 写 runbook 的约定

- **每条命令都要注明执行位置**（仓库根 / `crates/edge/`）—— 这个项目里 wrangler
  必须在 `crates/edge/` 跑，写错目录是最容易踩的一脚。
- **区分「已验证」与「未验证」**。没亲手跑通的步骤要显式标出来，
  不要让读者以为整篇都验证过 —— 那正是本仓库反复警告的「绿着的瞎子」。
- 需要凭据的步骤要写清**最小权限**，不要图省事写「给个全权 token」。
