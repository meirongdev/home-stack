output "worker_name" {
  description = "Worker 名字。"
  value       = module.home_stack.worker_name
}

output "deployed_version_id" {
  description = "当前吃 100% 流量的版本 id —— 回滚时要指定它的上一个。"
  value       = module.home_stack.version_id
}

output "url" {
  description = <<-EOT
    站点地址。没配自定义域名时是 null —— 那种情况下走
    `https://<worker_name>.<你的账号子域>.workers.dev`，账号子域在 Dashboard 看。
  EOT
  value       = module.home_stack.url
}
