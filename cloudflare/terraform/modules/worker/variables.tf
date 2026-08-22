# ─────────────────────────────────────────────────────────────────────────────
# 必填：调用方必须回答的问题
# ─────────────────────────────────────────────────────────────────────────────

variable "account_id" {
  description = "Cloudflare 账号 ID。"
  type        = string
}

variable "name" {
  description = <<-EOT
    Worker 名字，也决定 workers.dev 主机名：<name>.<账号子域>.workers.dev。

    ⚠️ **没有默认值是故意的。** Worker 名字在一个账号内唯一 ——
    同一账号里部署两份（比如一个 prod 一个 staging）必须起不同的名字，
    给默认值只会让第二次部署静默覆盖第一次。
  EOT
  type        = string

  validation {
    condition     = can(regex("^[a-z0-9][a-z0-9-]{0,62}$", var.name))
    error_message = "Worker 名字只能是小写字母、数字与连字符，且不以连字符开头。"
  }
}

# ─────────────────────────────────────────────────────────────────────────────
# 构建产物：**由调用方负责生成**
# ─────────────────────────────────────────────────────────────────────────────

variable "worker_build_dir" {
  description = <<-EOT
    worker-build 的产物目录，必须含 `index.js` 与 `index_bg.wasm`。

    ☠️ **本模块不会替你构建。** Terraform 不编 Rust、不建 Pagefind 索引。
    调用方要先在 home-stack 仓库里跑：
      cargo run -p xtask -- build-site
      cd crates/edge && worker-build --release

    路径要么是绝对路径，要么相对于**调用方的工作目录**（不是本模块目录）——
    模块内部不做 `path.module` 拼接，因为远程 source 消费时那个路径指向的是
    provider 的下载缓存目录，而产物不在那儿。
  EOT
  type        = string

  validation {
    condition     = length(trimspace(var.worker_build_dir)) > 0
    error_message = "worker_build_dir 不能为空。"
  }
}

variable "assets_dir" {
  description = <<-EOT
    资源层目录，由 `xtask build-site` 产出（内容是 Pagefind 索引）。

    ☠️ **绝不能指向 home-stack 的 `dist/`。** 那里是全站 HTML；Workers 是资源优先，
    HTML 进了资源层就会让每个页面都被静态命中、Router 永远不被唤起 ——
    站点看起来正常，但 SSR 那半个架构完全失效。
  EOT
  type        = string

  validation {
    condition     = length(trimspace(var.assets_dir)) > 0
    error_message = "assets_dir 不能为空。"
  }
}

# ─────────────────────────────────────────────────────────────────────────────
# 可选
# ─────────────────────────────────────────────────────────────────────────────

variable "compatibility_date" {
  description = <<-EOT
    Workers 运行时兼容日期。

    默认值跟的是 home-stack **实际测过**的那个日期 —— 它是代码的属性而不是
    部署环境的属性，所以默认值在这里而不是留给调用方猜。要改请自己验证。
  EOT
  type        = string
  default     = "2026-08-22"
}

variable "workers_dev_enabled" {
  description = "是否启用 <name>.workers.dev 子域。配了自定义域名后通常仍留着做冒烟测试。"
  type        = bool
  default     = true
}

variable "observability_enabled" {
  description = "是否开启 Workers 可观测性（调用日志 / 采样）。"
  type        = bool
  default     = true
}

variable "custom_domain" {
  description = <<-EOT
    方案 A：让 **Cloudflare 拥有 DNS**。填主机名（如 "stack.example.dev"）+ zone_name。

    ⚠️ Custom Domain 会自己创建 DNS 记录并签证书，且**不能建在已存在 CNAME 记录的
    主机名上**。所以调用方的 Terraform 里**不要**再声明同一条记录 ——
    两份 state 会互相打架，且带 prune 的一侧会试图删掉「不在它代码里」的那条记录。

    与 `route` 互斥。两个都留空 = 只用 workers.dev，完全不碰 DNS。
  EOT
  type = object({
    hostname  = string
    zone_name = string
  })
  default = null
}

variable "route" {
  description = <<-EOT
    方案 B：让**调用方拥有 DNS**。调用方自己建一条代理开启的记录，
    这里只绑路由（不碰 DNS）。

    适合「Cloudflare zone 的 DNS 已经由调用方的 Terraform 全量管理」的情况 ——
    那种项目通常不希望有任何记录游离在代码之外。

    与 `custom_domain` 互斥。
  EOT
  type = object({
    pattern = string # 如 "stack.example.dev/*"
    zone_id = string
  })
  default = null
}
