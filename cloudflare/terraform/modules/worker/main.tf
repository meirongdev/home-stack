# home-stack 的 Cloudflare Workers 部署，做成可复用子模块。
#
# 三个资源对应 Cloudflare 自己的三层模型：
#   Worker（长期实体）→ Version（不可变快照）→ Deployment（哪个版本吃流量）
# 这个分层正是它能被声明式管理、而不是只能 `wrangler deploy` 推一把的原因。

locals {
  # 互斥校验放 locals + precondition，而不是靠两个变量各自 validate ——
  # 单个变量的 validation 看不到另一个变量。
  dns_modes_chosen = (var.custom_domain != null ? 1 : 0) + (var.route != null ? 1 : 0)
}

resource "cloudflare_worker" "this" {
  account_id = var.account_id
  name       = var.name

  subdomain = {
    enabled = var.workers_dev_enabled
    # 预览 URL 会让每个版本都有一个公网地址。对一个只读的公开目录没有用处，
    # 少一个对外表面。调用方要的话自己 fork 这个模块。
    previews_enabled = false
  }

  observability = {
    enabled = var.observability_enabled
  }

  lifecycle {
    precondition {
      condition     = local.dns_modes_chosen <= 1
      error_message = "custom_domain 与 route 互斥：前者让 Cloudflare 拥有 DNS，后者让调用方拥有。只能选一个。"
    }
  }
}

# 不可变版本快照。文件内容一变 → provider 算出的哈希变 → 建一个新版本。
#
# ⚠️ 这里是**两个** module 而不是一个：worker-build 产的是 ESM + 单独的 wasm，
# `index.js` 里写着 `import X from "./index_bg.wasm"`。wasm 那个 module 的 `name`
# 必须和这个 import 说明符对得上（相对 main_module 解析）—— 写错的症状很难看：
# terraform apply 是绿的，Worker 启动即 500。
#
# 这两个名字是 worker-build 输出格式的属性，不是部署选项，所以不做成变量。
resource "cloudflare_worker_version" "this" {
  account_id = var.account_id
  worker_id  = cloudflare_worker.this.id

  compatibility_date = var.compatibility_date
  main_module        = "index.js"

  modules = [
    {
      name         = "index.js"
      content_type = "application/javascript+module"
      content_file = "${var.worker_build_dir}/index.js"
    },
    {
      name         = "index_bg.wasm"
      content_type = "application/wasm"
      content_file = "${var.worker_build_dir}/index_bg.wasm"
    },
  ]

  assets = {
    # provider 自己扫目录、算哈希、只上传变了的文件（5.11.0+）。
    directory = var.assets_dir

    config = {
      # 未命中资源**不要**由资源层兜底，交给 Worker 渲染 404 页。
      # 这是 Cloudflare 的默认值，显式写出来只为让这件事在代码里有据可查。
      not_found_handling = "none"

      # run_worker_first 先不设：它防的是「同名静态资源遮蔽动态路由」，
      # 而资源目录里只有 /pagefind/*，遮蔽不了任何东西。
    }
  }

  # 版本不可变：内容一变就要建新版本，而旧版本还被 deployment 引用着。
  # 不先建后毁的话，Terraform 会先毁掉正在服务流量的那个版本。
  lifecycle {
    create_before_destroy = true
  }
}

resource "cloudflare_workers_deployment" "this" {
  account_id  = var.account_id
  script_name = cloudflare_worker.this.name
  strategy    = "percentage"

  versions = [{
    version_id = cloudflare_worker_version.this.id
    percentage = 100
  }]
}

# 方案 A：Cloudflare 拥有 DNS。
resource "cloudflare_workers_custom_domain" "this" {
  count = var.custom_domain == null ? 0 : 1

  account_id = var.account_id
  hostname   = var.custom_domain.hostname
  zone_name  = var.custom_domain.zone_name
  service    = cloudflare_worker.this.name

  # ☠️ 这条 depends_on 不是保险，是**必需**的。本资源只引用 worker 的 `name`，
  # 依赖图里因此没有任何指向 version/deployment 的边 —— Terraform 会把它与
  # `cloudflare_worker_version`（上传资源层要十几秒）**并发**执行，而 Cloudflare
  # 拒绝给「还没有任何 deployment」的 Worker 挂自定义域名：
  #   400 / 100124 Cannot attach custom domain: Worker 'x' has no deployments
  # 2026-08-23 首次 apply 实撞：前 3 个资源建成、这一个失败。
  depends_on = [cloudflare_workers_deployment.this]
}

# 方案 B：调用方拥有 DNS，这里只绑路由。
resource "cloudflare_workers_route" "this" {
  count = var.route == null ? 0 : 1

  zone_id = var.route.zone_id
  pattern = var.route.pattern
  script  = cloudflare_worker.this.name

  # 同上那条边的理由。⚠️ 区别在证据强度：custom_domain 那个是 2026-08-23 实撞的，
  # route 这条**没有实测过**（本仓库走的是方案 A）。它同样只引用 worker 的 `name`、
  # 同样会与 version 并发，把顺序钉死不吃亏。
  depends_on = [cloudflare_workers_deployment.this]
}
