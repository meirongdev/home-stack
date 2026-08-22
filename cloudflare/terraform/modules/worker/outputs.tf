output "worker_name" {
  description = "Worker 名字（回滚、wrangler 调试命令都要用它）。"
  value       = cloudflare_worker.this.name
}

output "worker_id" {
  description = "Worker 的不可变 id。"
  value       = cloudflare_worker.this.id
}

output "version_id" {
  description = "当前吃 100% 流量的版本 id。"
  value       = cloudflare_worker_version.this.id
}

output "workers_dev_hostname" {
  description = <<-EOT
    workers.dev 主机名的**前半段**。⚠️ 账号子域（`<name>.<子域>.workers.dev` 里的
    `<子域>`）是账号级设置，provider 不返回它 —— 完整地址在 apply 输出或 Dashboard 看。
  EOT
  value       = var.workers_dev_enabled ? cloudflare_worker.this.name : null
}

output "url" {
  description = "自定义域名地址（没配则为 null）。"
  value = (
    var.custom_domain != null ? "https://${var.custom_domain.hostname}" :
    var.route != null ? "https://${replace(var.route.pattern, "/\\*$/", "")}" :
    null
  )
}
