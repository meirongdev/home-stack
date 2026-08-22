# 本仓库自己部署自己 —— 也就是 modules/worker 的**第一个消费者**。
# homelab 或任何其他 IaC 项目照同样的方式调用它，只是 source 换成 git 地址。
module "home_stack" {
  source = "./modules/worker"

  account_id = var.account_id
  name       = var.name

  # 构建产物路径。用 abspath + path.root 而不是相对路径：
  # 相对路径会随「terraform 从哪个目录被调用」而变，而 justfile、CI 与人手敲
  # 三者的工作目录并不总是一致。
  worker_build_dir = abspath("${path.root}/../../crates/edge/build")
  assets_dir       = abspath("${path.root}/../../public")

  custom_domain = var.custom_domain
  route         = var.route
}
