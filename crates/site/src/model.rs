//! 强类型内容模型 —— 分类法字段一律用 newtype，引用完整性在 `load` 阶段硬失败。
//!
//! 设计依据：`docs/decisions/typed-content-model-not-hugo.md`。

use serde::Deserialize;

/// 工具 slug（同时也是 `/tools/{slug}` 的路径段）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
pub struct Slug(String);

/// 分类法 id（必须存在于 categories.toml）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
pub struct CategoryId(String);

/// 被替换的 SaaS id（必须存在于 vendors.toml）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
pub struct VendorId(String);

/// FieldNote 集群 id（必须存在于 clusters.toml）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
pub struct ClusterId(String);

/// 一句话简介，≤125 字符（校验在 `xtask validate` 第 4 类）。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Summary(String);

/// SPDX license 表达式（如 `Apache-2.0`、`AGPL-3.0`）。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Spdx(String);

/// 日期，`YYYY-MM-DD`。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Date(String);

/// 出处文档 URL（FieldNote 必填，指向 homelab docs/decisions 或 docs/records）。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Url(String);

macro_rules! str_newtype {
    ($t:ty) => {
        impl $t {
            pub fn new(s: impl Into<String>) -> Self {
                Self(s.into())
            }
        }
        impl $t {
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }
        impl std::fmt::Display for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

str_newtype!(Slug);
str_newtype!(CategoryId);
str_newtype!(VendorId);
str_newtype!(ClusterId);
str_newtype!(Summary);
str_newtype!(Spdx);
str_newtype!(Date);
str_newtype!(Url);

/// 自托管部署方式，enum 而非自由文本。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum DeployKind {
    SingleBinary,
    Docker,
    Compose,
    HelmChart,
    Operator,
    DebRpm,
    SourceBuild,
}

/// FieldNote 四态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Status {
    Running,
    Retired,
    Rejected,
    Evaluating,
}

/// 实测足迹。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Footprint {
    #[serde(default)]
    pub mem_peak_7d: Option<String>,
    #[serde(default)]
    pub mem_limit: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

/// 一条踩坑记录：一句话 + 可选证据链接。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Gotcha {
    pub text: String,
    #[serde(default)]
    pub evidence: Option<Url>,
}

/// 一手运维证据（差异化核心，见 field-notes-as-differentiator）。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FieldNote {
    pub status: Status,
    #[serde(default)]
    pub since: Option<Date>,
    #[serde(default)]
    pub clusters: Vec<ClusterId>,
    #[serde(default)]
    pub footprint: Option<Footprint>,
    #[serde(default)]
    pub gotchas: Vec<Gotcha>,
    #[serde(default)]
    pub blind_spots: Vec<String>,
    /// Retired 时指向接替者。
    #[serde(default)]
    pub supersedes: Option<Slug>,
    /// 必填，非 Option —— 每个数字都必须能追回一份出处文档。
    pub decision: Url,
}

/// 上游链接。`repo` 非 `Option` —— 目录条目不给出处就只是二手转述，
/// 读者必须能一键跳到上游自己核对。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Links {
    pub repo: Url,
    #[serde(default)]
    pub site: Option<Url>,
    #[serde(default)]
    pub docs: Option<Url>,
}

/// 上游维护状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum UpstreamStatus {
    /// 正常迭代。
    Active,
    /// 只修关键问题 / 发布节奏明显停滞。
    Maintenance,
    /// 上游已归档，仓库只读。
    Archived,
}

/// 上游状态提醒 —— 只在「不写读者会踩坑」时出现（归档、停滞、易主、改名）。
///
/// `as_of` 与 `source` 都非 `Option`：状态是会过期的判断，
/// 不标核对日期与出处就没有可信度。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Upstream {
    pub status: UpstreamStatus,
    pub as_of: Date,
    pub note: String,
    pub source: Url,
}

/// 单条目录条目。前 8 项对齐常见目录条目，加 `field_note` 作为差异化的一半。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Tool {
    pub slug: Slug,
    pub name: String,
    pub summary: Summary,
    pub license: Spdx,
    pub language: String,
    #[serde(default)]
    pub categories: Vec<CategoryId>,
    #[serde(default)]
    pub replaces: Vec<VendorId>,
    #[serde(default)]
    pub self_host: Vec<DeployKind>,
    /// 上游是否也提供托管版。
    pub hosted: bool,
    /// 上游链接。非 `Option` —— 没有出处的条目不该存在。
    pub links: Links,
    /// 技术介绍：架构形状、存储依赖、协议、部署形态。
    /// 非 `Option` —— 「有条目但没有介绍」正是这个目录此前最大的缺口。
    pub detail: String,
    /// 跑起来之前必须先准备什么（对象存储 / 数据库 / 内核版本 / 另一个组件）。
    #[serde(default)]
    pub requires: Vec<String>,
    /// 上手前该知道的事：盲区、锁定、易踩的坑、许可证含义。
    #[serde(default)]
    pub watch: Vec<String>,
    #[serde(default)]
    pub upstream: Option<Upstream>,
    #[serde(default)]
    pub field_note: Option<FieldNote>,
}

/// 被替换的 SaaS 声明。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Vendor {
    pub id: VendorId,
    pub name: String,
}

/// 信号分类声明。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Category {
    pub id: CategoryId,
    pub name: String,
}

/// FieldNote 集群标识声明。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Cluster {
    pub id: ClusterId,
    pub name: String,
}

/// 一个上游仓库的活跃度快照。**不手写** —— 由 `xtask fetch` 生成，
/// 见 `docs/plans/2026-08-20-data-pipeline.md`：会过期的数字不进条目文件。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RepoStats {
    /// "github" / "gitlab" —— 不是所有条目都在 GitHub（GlitchTip 在 GitLab）。
    pub host: String,
    pub full_name: String,
    pub stars: u64,
    /// 可能缺失：GitHub 空仓库的 pushedAt 为 null；GitLab commits 接口抖动时
    /// fetch 也拿不到。对应 fail-soft 契约（见 data-pipeline 规格）——
    /// 若这里是必填，一次抓取失败就会让 validate / dev 硬失败。
    #[serde(default)]
    pub pushed_at: Option<Date>,
    #[serde(default)]
    pub latest_release: Option<String>,
    #[serde(default)]
    pub released_at: Option<Date>,
    /// 上游侧检测到的许可证，用来和手写的 `license` 交叉核对。
    #[serde(default)]
    pub license: Option<String>,
}

/// `content/generated/repo.json` 的内容。
///
/// `fetched_at` 必须渲染到页面上 —— fail-soft 的代价是数字会悄悄变旧，
/// 陈旧度必须可见，而不是藏在一个没人看的 CI 徽章里。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RepoIndex {
    pub fetched_at: Date,
    /// key 是工具 slug。
    pub repos: std::collections::BTreeMap<String, RepoStats>,
}

/// 已校验的完整目录。
#[derive(Debug, Clone)]
pub struct Catalog {
    pub vendors: Vec<Vendor>,
    pub categories: Vec<Category>,
    pub clusters: Vec<Cluster>,
    pub tools: Vec<Tool>,
    /// 生成数据。缺失是正常状态（还没跑过 fetch），页面上退化成不显示活跃度。
    pub repos: Option<RepoIndex>,
}

impl Catalog {
    pub fn tool(&self, slug: &Slug) -> Option<&Tool> {
        self.tools.iter().find(|t| &t.slug == slug)
    }

    pub fn category(&self, id: &CategoryId) -> Option<&Category> {
        self.categories.iter().find(|c| &c.id == id)
    }

    pub fn vendor(&self, id: &VendorId) -> Option<&Vendor> {
        self.vendors.iter().find(|v| &v.id == id)
    }

    /// 某工具的上游活跃度快照（没跑过 fetch 时为 `None`）。
    pub fn repo_stats(&self, slug: &Slug) -> Option<&RepoStats> {
        self.repos.as_ref()?.repos.get(slug.as_str())
    }

    /// 该分类下的工具（保持声明顺序）。
    pub fn tools_in_category<'a>(
        &'a self,
        id: &'a CategoryId,
    ) -> impl Iterator<Item = &'a Tool> + 'a {
        self.tools.iter().filter(move |t| t.categories.contains(id))
    }

    /// 替换了某 vendor 的工具。
    pub fn tools_replacing<'a>(&'a self, id: &'a VendorId) -> impl Iterator<Item = &'a Tool> + 'a {
        self.tools.iter().filter(move |t| t.replaces.contains(id))
    }
}
