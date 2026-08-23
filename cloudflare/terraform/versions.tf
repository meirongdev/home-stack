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

  # 状态后端：Cloudflare R2（S3 兼容），与 homelab 仓库
  # `docs/plans/architecture/2026-08-03-tf-state-r2.md` 的做法一致。
  #
  # ☠️ **endpoint 刻意不写在这里。** 它长成
  # `https://<账号 id>.r2.cloudflarestorage.com`，而**本仓库是公开的** ——
  # 账号 id 不算密钥，但也没有理由白送。它走一份 gitignored 的 `backend.hcl`：
  #
  #   cp backend.hcl.example backend.hcl   # 填进真实 endpoint
  #   terraform init -backend-config=backend.hcl
  #
  # 忘了带 `-backend-config` 的症状很清楚：init 直接报缺 endpoint，不会静默
  # 连到真 AWS S3（`skip_*` 那几个开关不足以让它猜出 R2 的地址）。
  # CI 里那份 backend.hcl 由 workflow 从 secret 现写（见 .github/workflows/deploy.yml）。
  #
  # ⏸ **无锁**：R2 没有 DynamoDB。Terraform 1.10+ 的 `use_lockfile`（S3 原生条件写）
  # 理论上能用，但**没实测过** —— 单人单机的现状下并发写不是真问题，想开就自己先验。
  #
  # ✅ **已启用**（2026-08-23，state 已迁入 R2）。两条随之改变的事：
  #   - 本地 `terraform.tfstate` **不再是真相源**：迁移把它**清空成 0 字节**，最后一份
  #     本地内容落到 `terraform.tfstate.backup`（都 gitignored）。⚠️ 看到那个 0 字节的
  #     文件别以为 state 丢了 —— 查现状用 `terraform state list`，读的是 R2。
  #   - 任何 terraform 命令都要 R2 的 S3 凭据（`AWS_ACCESS_KEY_ID` /
  #     `AWS_SECRET_ACCESS_KEY`）**和** `-backend-config=backend.hcl`。裸 `terraform init`
  #     会报缺 endpoint —— 用 `just init`，它带上了。
  # 迁移过程、凭据从哪来、失败时的绕法：runbooks/deploy-cloudflare.md 第 7 步。
  #
  backend "s3" {
    bucket = "terraform-backend"
    key    = "home-stack/cloudflare.tfstate"
    region = "auto"

    # ⚠️ R2 不是真 S3：这四个校验/签名步骤必须关，否则 init/plan 会在
    # 「校验凭据」「解析 region」「算 checksum」这些 AWS 专属环节上失败。
    skip_credentials_validation = true
    skip_region_validation      = true
    skip_requesting_account_id  = true
    skip_s3_checksum            = true
    use_path_style              = true
  }
}

# 凭据只从环境变量来：CLOUDFLARE_API_TOKEN。
# 不写进 provider 块、也不做成 terraform 变量 —— 变量会进 state，而 state 要被备份传阅。
provider "cloudflare" {}
