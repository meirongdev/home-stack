terraform {
  required_version = ">= 1.9"

  required_providers {
    cloudflare = {
      source = "cloudflare/cloudflare"
      # ⚠️ 下限不是随手写的：`assets.directory`（provider 自己扫目录、算哈希、
      # 分片上传静态资源）是 5.11.0 才有的。更低版本只能接受一个预先换来的上传
      # JWT —— 那就又要一段自定义脚本，IaC 的意义大半没了。
      version = ">= 5.11"
    }
  }
}

# ⚠️ **本模块刻意不含 `provider` 块，也不含 backend。**
# 那两样是**根模块**的职责 —— 谁部署，谁决定用哪个账号的凭据、state 放哪。
# 子模块里写 provider 会把调用方锁死在一种配置上，且无法被 for_each/count 复用。
