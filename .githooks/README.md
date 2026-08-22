# Git hooks

本仓库的钩子提交在 `.githooks/`，**不是** `.git/hooks/`（后者不随仓库走）。

```sh
.githooks/install     # 等价于 git config core.hooksPath .githooks
```

`core.hooksPath` 是本地配置，**每次新 clone 都要跑一次**。

## 分工：快的拦提交，慢的拦推送

| | `pre-commit` | `pre-push` |
|---|---|---|
| 目标 | 毫秒级，不打断心流 | 跑全套，宁可慢 |
| 范围 | 只看**暂存的 blob**（不是工作区） | 全仓库 |
| 卫生（密钥 / 体积 / 换行 / 空白） | ✅ | ✅ |
| 文档（命名 / 文首 / 链接 / 索引 / 计数） | ✅ | ✅ |
| `cargo fmt --check` | 改了 `.rs` 才跑 | ✅ |
| `cargo clippy -D warnings` / `cargo test` | ❌ | ✅ |
| `cargo check --target wasm32-unknown-unknown` | ❌ | ✅ `site` 与 `edge` 各一条 |
| `xtask validate` / `xtask render-diff` | ❌ | ✅ |

Rust 那几条在 `Cargo.toml` 出现之前自动跳过 —— 段 1 之前仓库只有文档，钩子照样能用。

## 为什么这两条 wasm 门禁在这里

`docs/decisions/dual-target-axum.md` 把两条纪律明确外包给了 CI：

1. **`cargo check --target wasm32-unknown-unknown`** —— `crates/site` 里每加一个依赖都要问
   「它编得到 wasm32 吗」，这条靠门禁兜住、不靠人记。
   ⚠️ `crates/edge` 要**单独查一条**：它的依赖挂在 `cfg(target_arch = "wasm32")` 下，
   宿主侧的 `fmt` / `clippy` / `test` 一行都看不到它 —— site 编得过但 edge 的胶水编不过，
   等于线上没有站。
2. **`xtask render-diff`** —— 不一致即硬失败。⚠️ 它当前比的是**两条内容路径**
   （磁盘 vs 构建期内嵌），**不覆盖** wasm32 运行时的输出；那半个判据还没兑现，
   见 `docs/ROADMAP.md` 开放项 13。

钩子是这些的**本地前哨**：在 dev 循环里就把漂移拦下，而不是等 CI 或线上才暴露。
CI 已经存在（`.github/workflows/ci.yml`，跑的是同一套加产物抽查）——
钩子可以 `--no-verify`，CI 不行。

## 检查脚本

| 脚本 | 查什么 |
|------|--------|
| `checks/hygiene.py` | 密钥模式（Tailscale / GitHub / AWS / Cloudflare / 私钥）、危险文件名（`.dev.vars`、`.env`、`*.pem`）、>2 MiB 大文件、`.editorconfig` 基线 |
| `checks/docs.py` | R2 命名、R3 文首必填字段、状态取值、ADR 必含章节、`plans/` 文件名日期与文首日期一致、全部相对链接与锚点、索引表与实际文件同步、「N 条 ADR」声明与实际篇数一致 |

R1（目录归属）判不了，仍然靠人守 —— 一篇文档该进 `decisions/` 还是 `reference/`
是语义判断，见 `docs/README.md` 的文档约定。

## 绕过

`git commit --no-verify` / `git push --no-verify`。绕过是留给「钩子本身出错」的，
不是留给「这次先不管」的 —— 前者请顺手修钩子，后者请在 commit message 里写明理由。

⚠️ 唯独密钥那条不要绕：凭据一旦进过 git 对象库就删不干净了，
正确的处理是**视同已泄露并轮换**。
