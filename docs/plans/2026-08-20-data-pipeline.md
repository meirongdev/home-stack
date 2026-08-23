# 数据管道规格：构建期抓取，运行时零出站

> 日期: 2026-08-20
> 状态: 🟡 GitHub 侧**已完成**（2026-08-21；夜间 CI 自 2026-08-22 起在跑）；
> Prometheus 侧仍**待实施**
> —— `xtask fetch` 已产出 `content/generated/repo.json`（stars / pushed_at /
> latest_release / license，**95 条** —— 97 个条目里有 2 条上游不在 GitHub/GitLab，
> fetch 会跳过并告警），页面显示 `fetched_at`；`xtask validate` 在数据超过 7 天时
> warning。footprint.json 与 Tailscale 那一侧尚未实施。
> 结论：`xtask fetch` 在构建期从 GitHub GraphQL + homelab Prometheus 抓数，
> 产出 committed JSON，`include_str!` 编进 WASM。
> ⚠️ **fetch 必须 fail-soft，validate 必须硬失败** —— 两者性质不同。

## 为什么全放在构建期

`crates/xtask` 是**原生** crate，不受 wasm32 约束，可以随便用 tokio / reqwest / rustls。
`crates/site` 零 I/O，运行时因此不发任何出站请求。这同时是性能、是 Workers CPU 预算的护栏，
也避免了把公网站点的**可用性**绑上 homelab（见下方 fail-soft 一节）。

## 两个来源，一个产物

```
xtask fetch
  ├─ GitHub GraphQL  → stars, pushedAt, latestRelease, licenseInfo
  │                     └→ freshness 由 pushedAt 算出，不再手工标
  ├─ Prometheus API  → 实测 footprint（经 Tailscale 进 homelab）
  └─ → content/generated/{repo,footprint}.json   (committed)
```

产物 committed 进仓库，不是构建时临时物。理由：一次抓取失败不该让站点变空，
且 git 历史顺带成为「这些数字什么时候变的」的记录。

### GitHub 侧

用 GraphQL 而非 REST：一次请求拿全部仓库的四个字段，避免 40 次 REST 调用打配额。
`freshness` 由 `pushedAt` 算出，取代需要手工维护的 freshness 徽章。

### Prometheus 侧

经 Tailscale 进 homelab 查中枢 Prometheus。示例查询：

```promql
# 每个可观测性组件自己的内存足迹
max_over_time(
  container_memory_working_set_bytes{namespace="monitoring", container!=""}[7d]
)

# Prometheus 自己产生的 series 数
prometheus_tsdb_head_series
```

⚠️ **只抓能公开的聚合数字**（内存峰值、series 数），不抓拓扑、主机名、标签值 ——
产物会上公网。

## 夜间 CI

```yaml
# .github/workflows/refresh.yml
on:
  schedule: [{ cron: "17 3 * * *" }]   # 03:17 UTC
  workflow_dispatch: {}

steps:
  - uses: actions/checkout@v4
  - uses: tailscale/github-action@v4.1.3
    with:
      oauth-client-id: ${{ secrets.TS_OAUTH_CLIENT_ID }}
      oauth-secret:    ${{ secrets.TS_OAUTH_SECRET }}
      tags:            tag:ci
  - run: cargo run -p xtask -- fetch      # fail-soft
  - run: cargo run -p xtask -- validate   # 硬失败
  - run: npx pagefind --site dist
  - run: npx wrangler deploy
```

**前置依赖**：`tag:ci` 需在 homelab 仓库的 `tailscale/terraform` ACL 里放行到中枢
Prometheus。这是跨仓库依赖，实施时要在两边都留注释，否则日后有人清 ACL 会把这条打断
且症状是「站点数字停止更新」——不会有人立刻发现。

**cron 选 03:17 UTC**：避开 homelab readlist 夜间管道的 01:05–01:40 窗口。
那台 VM 内存吃紧（Prometheus 峰值已到 limit 的 88%），别在它最忙的时候再去压。

## ⚠️ fetch 必须 fail-soft

这是本规格里最重要的一条。

`fetch` 拉不到 Prometheus（tailnet 断、VM 重启、Prometheus 又撞内存、ACL 被清）时，
**必须保留上一次 committed 的 `footprint.json` 并只发 warning，退出码 0**。

理由：一次 homelab 抖动不该让公开站点构建失败。反过来说，如果 `fetch` 硬失败，
就等于把一个零集群开销的公网站点重新绑回了 homelab 的可用性 ——
那样选 Cloudflare 的意义就废掉了一半
（见 [../decisions/cloudflare-workers-not-pages.md](../decisions/cloudflare-workers-not-pages.md)）。

**对照**：`validate` 必须**硬失败**。那是内容错误，本来就该拦住
（见 [2026-08-20-content-model.md](2026-08-20-content-model.md)）。
两条规则方向相反，别混。

### 陈旧度必须可见

fail-soft 的代价是数字可能悄悄变旧 —— 这正是 homelab
`readlist` 踩过的那类坑（管道全挂但页面照常返回 200，榜单在悄悄变旧，
探针绿、Uptime Kuma 绿、首页绿）。

所以产物里必须带 `fetched_at`，页面上**显示**数据抓取时间；
超过阈值（建议 7 天）在页面上标注为陈旧。让它在 UI 上可见，
而不是依赖一个没人看的 CI 状态徽章。

## Consequences

**得到**：freshness 不再手工标；footprint 是真实测量而非估算；运行时零出站请求。

**付出**：跨仓库依赖一条（Tailscale ACL）；GitHub secrets 两个（TS OAuth）；
数字的准确性取决于 cron 是否还在跑 —— 由上面的陈旧度显示兜住。

## 未决

- **是否值得引入 Prometheus 侧**。GitHub 侧收益明确（freshness 自动化）；
  Prometheus 侧只服务少数几条 FieldNote 的 footprint 字段，却引入了跨仓库 ACL 依赖。
  实施时先做 GitHub 侧，Prometheus 侧观察 FieldNote 实际数量再定 ——
  如果最终只有 5–10 条，手写 + 季度人工核对可能更划算。
