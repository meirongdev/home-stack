//! 从 TOML 字符串构建并校验 `Catalog`。
//!
//! 本模块是纯逻辑、零 I/O：输入是 `(path, raw)` 字符串对，文件读取由
//! `crates/dev` / `crates/xtask` 负责。四类校验全部**硬失败**：
//! 1. `replaces` / `categories` / `clusters` 引用必须已声明
//! 2. 孤儿分类：声明了但零条目引用
//! 3. slug 唯一
//! 4. `summary` ≤125 字符
//!
//! 报错形态贴近 rustc，带 `help` / `note`，用 `toml::Spanned` 拿精确行列号。

use crate::model::*;
use serde::Deserialize;
use toml::Spanned;

pub const SUMMARY_MAX_CHARS: usize = 125;

/// 一条校验问题。`line` / `col` / `end_col` 为 1-based 字符列。
#[derive(Debug, Clone)]
pub struct Issue {
    pub path: String,
    pub line: usize,
    pub col: usize,
    pub end_col: usize,
    pub source_line: Option<String>,
    pub message: String,
    pub help: Option<String>,
    pub note: Option<String>,
}

impl Issue {
    fn new(
        path: &str,
        raw: &str,
        span: (usize, usize),
        message: impl Into<String>,
        help: Option<String>,
        note: Option<String>,
    ) -> Self {
        let (line, col) = offset_to_line_col(raw, span.0);
        let (_, end_col) = offset_to_line_col(raw, span.1);
        let source_line = line_text(raw, line);
        Issue {
            path: path.to_string(),
            line,
            col,
            end_col,
            source_line,
            message: message.into(),
            help,
            note,
        }
    }
}

/// 字节偏移 → (1-based 行, 1-based 字符列)。
fn offset_to_line_col(raw: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(raw.len());
    let before = &raw[..offset];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let col = raw[line_start..offset].chars().count() + 1;
    (line, col)
}

fn line_text(raw: &str, line: usize) -> Option<String> {
    raw.lines().nth(line - 1).map(str::to_string)
}

/// 解析时使用的带 span 包装层（不进入最终模型）。
#[derive(Deserialize)]
struct VendorsFile {
    #[serde(rename = "vendor")]
    vendor: Vec<SpannedVendor>,
}

#[derive(Deserialize)]
struct SpannedVendor {
    id: Spanned<VendorId>,
    name: String,
}

#[derive(Deserialize)]
struct CategoriesFile {
    #[serde(rename = "category")]
    category: Vec<SpannedCategory>,
}

#[derive(Deserialize)]
struct SpannedCategory {
    id: Spanned<CategoryId>,
    name: String,
}

#[derive(Deserialize)]
struct ClustersFile {
    #[serde(rename = "cluster")]
    cluster: Vec<SpannedCluster>,
}

#[derive(Deserialize)]
struct SpannedCluster {
    id: Spanned<ClusterId>,
    name: String,
}

#[derive(Deserialize)]
struct SpannedTool {
    slug: Spanned<Slug>,
    name: String,
    summary: Spanned<Summary>,
    license: Spanned<Spdx>,
    language: String,
    #[serde(default)]
    categories: Vec<Spanned<CategoryId>>,
    #[serde(default)]
    replaces: Vec<Spanned<VendorId>>,
    #[serde(default)]
    self_host: Vec<DeployKind>,
    hosted: bool,
    links: Links,
    detail: String,
    #[serde(default)]
    requires: Vec<String>,
    #[serde(default)]
    watch: Vec<String>,
    #[serde(default)]
    upstream: Option<Upstream>,
    #[serde(default)]
    field_note: Option<SpannedFieldNote>,
}

#[derive(Deserialize)]
struct SpannedFieldNote {
    status: Status,
    #[serde(default)]
    since: Option<Date>,
    #[serde(default)]
    clusters: Vec<Spanned<ClusterId>>,
    #[serde(default)]
    footprint: Option<Footprint>,
    #[serde(default)]
    gotchas: Vec<Gotcha>,
    #[serde(default)]
    blind_spots: Vec<String>,
    #[serde(default)]
    supersedes: Option<Spanned<Slug>>,
    decision: Url,
}

fn span_start<T>(s: &Spanned<T>) -> (usize, usize) {
    let r = s.span();
    let a = r.start;
    let b = r.end.max(a + 1);
    (a, b)
}

/// 从 `(path, raw)` 对构建并校验 `Catalog`。
///
/// 约定：文件名决定文件种类 —— `vendors.toml` / `categories.toml` / `clusters.toml`
/// 是声明文件，其余（`tools/*.toml`）是工具条目。
pub fn load(sources: &[(&str, &str)]) -> Result<Catalog, Vec<Issue>> {
    let mut issues = Vec::new();

    let mut vendors: Vec<Vendor> = Vec::new();
    let mut categories: Vec<Category> = Vec::new();
    let mut clusters: Vec<Cluster> = Vec::new();
    let mut tools: Vec<(String, String, Tool)> = Vec::new(); // (path, raw, tool)
    let mut repos: Option<RepoIndex> = None;

    for (path, raw) in sources {
        let base = path.rsplit('/').next().unwrap_or(path);
        match base {
            "vendors.toml" => match toml::from_str::<VendorsFile>(raw) {
                Ok(f) => {
                    vendors = f
                        .vendor
                        .into_iter()
                        .map(|v| Vendor {
                            id: v.id.into_inner(),
                            name: v.name,
                        })
                        .collect()
                }
                Err(e) => issues.push(parse_issue(path, raw, e, "vendors.toml 解析失败")),
            },
            "categories.toml" => match toml::from_str::<CategoriesFile>(raw) {
                Ok(f) => {
                    categories = f
                        .category
                        .into_iter()
                        .map(|c| Category {
                            id: c.id.into_inner(),
                            name: c.name,
                        })
                        .collect()
                }
                Err(e) => issues.push(parse_issue(path, raw, e, "categories.toml 解析失败")),
            },
            "clusters.toml" => match toml::from_str::<ClustersFile>(raw) {
                Ok(f) => {
                    clusters = f
                        .cluster
                        .into_iter()
                        .map(|c| Cluster {
                            id: c.id.into_inner(),
                            name: c.name,
                        })
                        .collect()
                }
                Err(e) => issues.push(parse_issue(path, raw, e, "clusters.toml 解析失败")),
            },
            // 生成数据（`xtask fetch` 产物）。缺失是正常状态；**格式错**是仓库缺陷，硬失败。
            "repo.json" => match serde_json::from_str::<RepoIndex>(raw) {
                Ok(idx) => repos = Some(idx),
                Err(e) => issues.push(Issue::new(
                    path,
                    raw,
                    (0, 1),
                    format!("repo.json 解析失败：{e}"),
                    Some(
                        "这是 `cargo run -p xtask -- fetch` 的产物，手工改坏了就重新跑一次".into(),
                    ),
                    None,
                )),
            },
            _ => match toml::from_str::<SpannedTool>(raw) {
                Ok(t) => tools.push((path.to_string(), raw.to_string(), spanned_tool_to_tool(t))),
                Err(e) => issues.push(parse_issue(path, raw, e, "工具条目解析失败")),
            },
        }
    }

    let declared_vendors: Vec<&VendorId> = vendors.iter().map(|v| &v.id).collect();
    let declared_categories: Vec<&CategoryId> = categories.iter().map(|c| &c.id).collect();
    let declared_clusters: Vec<&ClusterId> = clusters.iter().map(|c| &c.id).collect();

    // ── 校验 1：引用必须已声明 ─────────────────────────────────────────
    for (path, raw, _tool) in &tools {
        // 用带 span 的字段重新解析以拿行列号
        if let Ok(st) = toml::from_str::<SpannedTool>(raw) {
            for c in &st.categories {
                if !declared_categories.contains(&&c.get_ref().clone()) {
                    issues.push(Issue::new(
                        path,
                        raw,
                        span_start(c),
                        format!("未声明的分类引用 `{}`", c.as_ref()),
                        Some(format!(
                            "categories.toml 已声明: {}",
                            join_ids(&declared_categories)
                        )),
                        Some("Hugo 在这里会静默生成一个空的分类页面并正常上线".into()),
                    ));
                }
            }
            for r in &st.replaces {
                if !declared_vendors.contains(&&r.get_ref().clone()) {
                    issues.push(Issue::new(
                        path,
                        raw,
                        span_start(r),
                        format!("未声明的 vendor 引用 `{}`", r.as_ref()),
                        Some(format!(
                            "vendors.toml 已声明: {}",
                            join_ids(&declared_vendors)
                        )),
                        Some(format!(
                            "Hugo 在这里会静默生成一个空的 /replaces/{}/ 页面并正常上线",
                            r.as_ref()
                        )),
                    ));
                }
            }
            if let Some(fn_) = &st.field_note {
                for cl in &fn_.clusters {
                    if !declared_clusters.contains(&&cl.get_ref().clone()) {
                        issues.push(Issue::new(
                            path,
                            raw,
                            span_start(cl),
                            format!("未声明的集群引用 `{}`", cl.as_ref()),
                            Some(format!(
                                "clusters.toml 已声明: {}",
                                join_ids(&declared_clusters)
                            )),
                            Some("FieldNote.clusters 必须指向已声明的集群".into()),
                        ));
                    }
                }
                if let Some(sup) = &fn_.supersedes {
                    if !tools.iter().any(|(_, _, t)| &t.slug == sup.get_ref()) {
                        issues.push(Issue::new(
                            path,
                            raw,
                            span_start(sup),
                            format!("supersedes 指向不存在的工具 `{}`", sup.as_ref()),
                            Some("supersedes 必须指向目录中已有的 slug".into()),
                            None,
                        ));
                    }
                }
            }
        }
    }

    // ── 校验 2：孤儿分类 ───────────────────────────────────────────────
    let categories_raw = sources
        .iter()
        .find(|(p, _)| p.ends_with("categories.toml"))
        .map(|(_, r)| *r);
    for cat in &categories {
        let used = tools.iter().any(|(_, _, t)| t.categories.contains(&cat.id));
        if !used {
            let line_no = categories_raw
                .and_then(|raw| {
                    raw.lines().position(|l| {
                        l.trim().starts_with("id =") && l.contains(&cat.id.to_string())
                    })
                })
                .unwrap_or(0)
                + 1;
            let source_line = categories_raw
                .and_then(|raw| raw.lines().nth(line_no - 1))
                .map(str::to_string);
            issues.push(Issue {
                path: "content/categories.toml".to_string(),
                line: line_no,
                col: 1,
                end_col: 1,
                source_line,
                message: format!("孤儿分类 `{}`：声明了但零条目引用", cat.id),
                help: Some("给某个工具加上该分类，或从 categories.toml 删掉它".into()),
                note: Some("Hugo 会生成一个空列表页".into()),
            });
        }
    }

    // ── 校验 3：slug 唯一 ──────────────────────────────────────────────
    let mut seen: Vec<&Slug> = Vec::new();
    for (path, raw, tool) in &tools {
        if let Some(_prev) = seen.iter().find(|s| **s == &tool.slug) {
            issues.push(Issue::new(
                path,
                raw,
                parse_slug_span(raw).unwrap_or((0, 1)),
                format!("slug `{}` 重复", tool.slug),
                Some("slug 必须全局唯一".into()),
                Some("Hugo 里后者覆盖前者或产生冲突路径".into()),
            ));
        } else {
            seen.push(&tool.slug);
        }
    }

    // ── 校验 4：summary ≤125 字符 ─────────────────────────────────────
    for (path, raw, tool) in &tools {
        if tool.summary.as_str().chars().count() > SUMMARY_MAX_CHARS {
            let span = if let Ok(st) = toml::from_str::<SpannedTool>(raw) {
                span_start(&st.summary)
            } else {
                (0, 1)
            };
            issues.push(Issue::new(
                path,
                raw,
                span,
                format!(
                    "summary 超长（{} 字符，上限 {}）",
                    tool.summary.as_str().chars().count(),
                    SUMMARY_MAX_CHARS
                ),
                Some(format!("summary 不能超过 {} 字符", SUMMARY_MAX_CHARS)),
                Some("卡片会被撑破版，只在视觉上暴露".into()),
            ));
        }
    }

    // ── 生成数据的引用完整性：repo.json 的 key 必须是真实条目的 slug ─────
    if let Some(idx) = &repos {
        let known: Vec<&str> = tools.iter().map(|(_, _, t)| t.slug.as_str()).collect();
        for slug in idx.repos.keys() {
            if !known.contains(&slug.as_str()) {
                issues.push(Issue {
                    path: "content/generated/repo.json".into(),
                    line: 1,
                    col: 1,
                    end_col: 1,
                    source_line: None,
                    message: format!("repo.json 里的 `{slug}` 不是已有条目的 slug"),
                    help: Some("条目改名或删除后要重跑 `xtask fetch`".into()),
                    note: Some("放过它就等于页面上挂着一份对不上任何条目的数字".into()),
                });
            }
        }
    }

    if issues.is_empty() {
        Ok(Catalog {
            vendors,
            categories,
            clusters,
            tools: tools.into_iter().map(|(_, _, t)| t).collect(),
            repos,
        })
    } else {
        Err(issues)
    }
}

fn parse_issue(path: &str, raw: &str, e: toml::de::Error, msg: &str) -> Issue {
    let span = match e.span() {
        Some(r) => (r.start, r.end.max(r.start + 1)),
        None => (0, 1),
    };
    Issue::new(
        path,
        raw,
        span,
        format!("{msg}: {}", e.message()),
        None,
        None,
    )
}

fn spanned_tool_to_tool(st: SpannedTool) -> Tool {
    Tool {
        slug: st.slug.into_inner(),
        name: st.name,
        summary: st.summary.into_inner(),
        license: st.license.into_inner(),
        language: st.language,
        categories: st.categories.into_iter().map(|c| c.into_inner()).collect(),
        replaces: st.replaces.into_iter().map(|r| r.into_inner()).collect(),
        self_host: st.self_host,
        hosted: st.hosted,
        links: st.links,
        detail: st.detail,
        requires: st.requires,
        watch: st.watch,
        upstream: st.upstream,
        field_note: st.field_note.map(|f| FieldNote {
            status: f.status,
            since: f.since,
            clusters: f.clusters.into_iter().map(|c| c.into_inner()).collect(),
            footprint: f.footprint,
            gotchas: f.gotchas,
            blind_spots: f.blind_spots,
            supersedes: f.supersedes.map(|s| s.into_inner()),
            decision: f.decision,
        }),
    }
}

fn parse_slug_span(raw: &str) -> Option<(usize, usize)> {
    toml::from_str::<SpannedTool>(raw)
        .ok()
        .map(|t| span_start(&t.slug))
}

fn join_ids(ids: &[&(impl std::fmt::Display + ?Sized)]) -> String {
    ids.iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vendors_toml() -> &'static str {
        "[[vendor]]\nid = \"datadog\"\nname = \"Datadog\"\n"
    }
    fn categories_toml() -> &'static str {
        "[[category]]\nid = \"metrics\"\nname = \"指标\"\n[[category]]\nid = \"logs\"\nname = \"日志\"\n"
    }
    fn clusters_toml() -> &'static str {
        "[[cluster]]\nid = \"homelab\"\nname = \"homelab\"\n"
    }
    fn tool(categories: &str, replaces: &str) -> String {
        format!(
            "slug = \"prometheus\"\nname = \"Prometheus\"\nsummary = \"CNCF 毕业的指标事实标准。\"\nlicense = \"Apache-2.0\"\nlanguage = \"Go\"\ncategories = {categories}\nreplaces = {replaces}\nself_host = [\"SingleBinary\", \"Docker\"]\nhosted = false\ndetail = \"拉模型 + PromQL + 本地 TSDB。\"\n\n[links]\nrepo = \"https://github.com/prometheus/prometheus\"\n"
        )
    }

    fn sources() -> Vec<(&'static str, String)> {
        vec![
            ("content/vendors.toml", vendors_toml().to_string()),
            ("content/categories.toml", categories_toml().to_string()),
            ("content/clusters.toml", clusters_toml().to_string()),
            (
                "content/tools/prometheus.toml",
                tool("[\"metrics\", \"logs\"]", "[\"datadog\"]"),
            ),
        ]
    }

    #[test]
    fn valid_catalog_loads() {
        let s = sources();
        let refs: Vec<(&str, &str)> = s.iter().map(|(p, r)| (*p, r.as_str())).collect();
        let cat = load(&refs).expect("应当加载成功");
        assert_eq!(cat.tools.len(), 1);
        assert_eq!(cat.vendors.len(), 1);
        assert_eq!(cat.categories.len(), 2);
    }

    #[test]
    fn undeclared_vendor_fails_with_line() {
        let mut s = sources();
        s[3].1 = tool("[\"metrics\", \"logs\"]", "[\"datadog-logs\"]");
        let refs: Vec<(&str, &str)> = s.iter().map(|(p, r)| (*p, r.as_str())).collect();
        let err = load(&refs).expect_err("应失败");
        let issue = err
            .iter()
            .find(|i| i.message.contains("vendor"))
            .expect("应有 vendor 报错");
        assert!(issue.line >= 1);
        assert!(issue.help.as_ref().unwrap().contains("vendors.toml 已声明"));
    }

    #[test]
    fn repo_json_with_null_pushed_at_loads() {
        // fail-soft 契约（docs/plans/2026-08-20-data-pipeline.md）：
        // GitLab commits 接口抖动 / GitHub 空仓库会把 pushed_at 写成 null，
        // 这不该让 validate / dev 硬失败。
        let mut s = sources();
        s.push((
            "content/generated/repo.json",
            r#"{
  "fetched_at": "2026-08-21",
  "repos": {
    "prometheus": {
      "host": "github",
      "full_name": "prometheus/prometheus",
      "stars": 65768,
      "pushed_at": null
    }
  }
}"#
            .to_string(),
        ));
        let refs: Vec<(&str, &str)> = s.iter().map(|(p, r)| (*p, r.as_str())).collect();
        let cat = load(&refs).expect("pushed_at: null 应当可加载");
        let idx = cat.repos.expect("repo.json 应解析成功");
        let stats = idx.repos.get("prometheus").expect("应有 prometheus 条目");
        assert!(stats.pushed_at.is_none(), "null 应反序列化为 None");
    }

    #[test]
    fn orphan_category_fails() {
        let mut s = sources();
        s[1].1 = "[[category]]\nid = \"metrics\"\nname = \"指标\"\n[[category]]\nid = \"orphan\"\nname = \"孤儿\"\n".to_string();
        let refs: Vec<(&str, &str)> = s.iter().map(|(p, r)| (*p, r.as_str())).collect();
        let err = load(&refs).expect_err("孤儿分类应失败");
        assert!(err.iter().any(|i| i.message.contains("孤儿分类")));
    }

    #[test]
    fn duplicate_slug_fails() {
        let mut s = sources();
        s.push((
            "content/tools/prometheus-dup.toml",
            tool("[\"metrics\"]", "[\"datadog\"]"),
        ));
        let refs: Vec<(&str, &str)> = s.iter().map(|(p, r)| (*p, r.as_str())).collect();
        let err = load(&refs).expect_err("重复 slug 应失败");
        assert!(err.iter().any(|i| i.message.contains("重复")));
    }

    #[test]
    fn long_summary_fails() {
        let mut s = sources();
        s[3].1 = tool("[\"metrics\", \"logs\"]", "[\"datadog\"]").replace(
            "summary = \"CNCF 毕业的指标事实标准。\"",
            &format!("summary = \"{}\"", "很".repeat(200)),
        );
        let refs: Vec<(&str, &str)> = s.iter().map(|(p, r)| (*p, r.as_str())).collect();
        let err = load(&refs).expect_err("超长 summary 应失败");
        assert!(err.iter().any(|i| i.message.contains("summary 超长")));
    }

    #[test]
    fn undeclared_category_and_cluster_fail() {
        let mut s = sources();
        s[3].1 = tool("[\"nope\"]", "[\"datadog\"]");
        s.push((
            "content/tools/otel.toml",
            "slug = \"otel\"\nname = \"OTel\"\nsummary = \"采集器\"\nlicense = \"Apache-2.0\"\nlanguage = \"Go\"\ncategories = [\"metrics\"]\nreplaces = [\"datadog\"]\nself_host = [\"Docker\"]\nhosted = false\ndetail = \"采集器\"\n\n[links]\nrepo = \"https://github.com/open-telemetry/opentelemetry-collector\"\n\n[field_note]\nstatus = \"Running\"\nclusters = [\"missing-cluster\"]\ndecision = \"https://example.com/x\"\n".to_string(),
        ));
        let refs: Vec<(&str, &str)> = s.iter().map(|(p, r)| (*p, r.as_str())).collect();
        let err = load(&refs).expect_err("未声明引用应失败");
        assert!(err.iter().any(|i| i.message.contains("分类引用")));
        assert!(err.iter().any(|i| i.message.contains("集群引用")));
    }
}
