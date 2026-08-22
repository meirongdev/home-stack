variable "account_id" {
  description = "Cloudflare 账号 ID（Dashboard 右侧栏可复制）。用 TF_VAR_account_id 传。"
  type        = string
}

variable "name" {
  description = "Worker 名字。也决定 workers.dev 主机名。"
  type        = string
  default     = "home-stack"
}

variable "custom_domain" {
  description = <<-EOT
    可选。方案 A：让 Cloudflare 拥有这条 DNS 记录。
    例：{ hostname = "stack.meirong.dev", zone_name = "meirong.dev" }

    ⚠️ 填了它就意味着 homelab 仓库的 cloudflare/terraform 里**不要**再声明同一条记录。
    与 route 互斥，取舍见 docs/runbooks/deploy-cloudflare.md。
  EOT
  type = object({
    hostname  = string
    zone_name = string
  })
  default = null
}

variable "route" {
  description = <<-EOT
    可选。方案 B：DNS 记录由别处（如 homelab 的 Terraform）拥有，这里只绑路由。
    例：{ pattern = "stack.meirong.dev/*", zone_id = "…" }
  EOT
  type = object({
    pattern = string
    zone_id = string
  })
  default = null
}
