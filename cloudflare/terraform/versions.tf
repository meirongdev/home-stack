# 根模块：决定用哪个账号的凭据、state 放哪，然后调用 modules/worker。
# 部署逻辑本身不在这里 —— 那样 homelab 之类的项目才能只取模块、自己当根。
terraform {
  required_version = ">= 1.9"

  required_providers {
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 5.23"
    }
  }

  # 状态后端。首次部署用本地 state 最省事；**CI 部署必须换成远端**
  # （干净 runner + 本地 state = 每次都从空开始，会去创建已存在的 Worker）。
  # 下面是 R2（S3 兼容）的写法，与 homelab 仓库
  # `docs/plans/architecture/2026-08-03-tf-state-r2.md` 的做法一致。
  #
  # backend "s3" {
  #   bucket                      = "<tf-state-bucket>"
  #   key                         = "home-stack/cloudflare.tfstate"
  #   region                      = "auto"
  #   endpoints                   = { s3 = "https://<accountid>.r2.cloudflarestorage.com" }
  #   skip_credentials_validation = true
  #   skip_region_validation      = true
  #   skip_requesting_account_id  = true
  #   skip_s3_checksum            = true
  #   use_path_style              = true
  # }
}

# 凭据只从环境变量来：CLOUDFLARE_API_TOKEN。
# 不写进 provider 块、也不做成 terraform 变量 —— 变量会进 state，而 state 要被备份传阅。
provider "cloudflare" {}
