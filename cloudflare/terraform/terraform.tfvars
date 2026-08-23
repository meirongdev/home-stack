# home-stack 自己部署自己的输入。⚠️ 两个值**刻意不在这里**：
#   - token   → 只走环境变量 CLOUDFLARE_API_TOKEN（写进 tfvars 会进 state）
#   - account_id → 走 TF_VAR_account_id
# 见 docs/runbooks/deploy-cloudflare.md 第 1 步。

# 方案 A：DNS 归属给 Cloudflare —— cloudflare_workers_custom_domain 自己建记录并签证书。
#
# ☠️ 这是一条**跨仓库依赖**：meirong.dev 这个 zone 的其余 DNS 由 homelab 仓库管
# （cloudflare/terraform + 两个 external-dns 实例）。这条记录**只由本仓库声明**，
# homelab 那边不要再写第二份。
#
# 2026-08-23 实测确认对面不会误删它：两个 external-dns 都是 `policy: upsert-only`
# （只增改、从不删），homelab 的 cloudflare_dns_record.subdomains 是空 map 的 for_each，
# 而 terraform 本身不 prune 不在自己 state 里的记录。
custom_domain = {
  hostname  = "stack.meirong.dev"
  zone_name = "meirong.dev"
}
