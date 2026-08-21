//! Maud 模板 —— 编译期检查的 HTML。所有渲染都是纯函数（零 I/O）。
//!
//! 两条版式纪律：
//! 1. **能点的才做成 chip。** 事实一律是纯文本 —— 圆角小块只用于真链接，
//!    读者不必试点才知道哪个能点。
//! 2. **卡片内部不放链接。** 整张卡片本身就是 `<a>`；嵌套 `<a>` 是非法 HTML，
//!    浏览器会按 adoption agency 把内层拆出去，让 tag 行变成独立的网格单元
//!    （回归测试见本文件 `cards_have_no_nested_anchor`）。

use crate::model::{Catalog, Category, RepoStats, Status, Tool, Upstream, UpstreamStatus, Vendor};
use maud::{html, Markup};

/// 页面外壳：head + 顶部导航 + 主区 + 页脚。
fn layout(title: &str, body: Markup) -> Markup {
    html! {
        (maud::DOCTYPE)
        html lang="zh-CN" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " — Probe Directory" }
                style {
                    (include_str!("templates.css"))
                }
            }
            body {
                header.nav {
                    .nav-inner {
                        a.brand href="/" { "Probe Directory" }
                    }
                }
                main { (body) }
                footer {
                    .footer-inner {
                        "每条条目都链回上游仓库与文档 —— 字段可以核对，取舍自己判断。"
                    }
                }
            }
        }
    }
}

/// 面包屑 —— 子页唯一的返回路径（顶栏只留 brand）。
///
/// 只放能点的段：末段「当前页」一律省掉，那是 3 行之下的 h1 已经说过的话。
fn crumb(parents: &[(String, String)]) -> Markup {
    html! {
        nav.crumb {
            a href="/" { "← 目录" }
            @for (href, label) in parents {
                span.sep { "/" }
                a href=(href) { (label) }
            }
        }
    }
}

fn chip(href: &str, label: &str) -> Markup {
    html! {
        a.chip href=(href) { (label) }
    }
}

/// 站外链接。和站内 chip 必须长得不一样 —— 点下去会离开本站，
/// 读者有权在点之前就知道这件事。
fn chip_ext(href: &str, label: &str) -> Markup {
    html! {
        a.chip.chip-ext href=(href) rel="noreferrer" {
            (label)
            span.ext-mark aria-hidden="true" { "↗" }
        }
    }
}

/// `https://github.com/prometheus/prometheus` → `github.com/prometheus/prometheus`。
///
/// 链接文字用仓库路径本身而不是「仓库」两个字：路径能一眼认出是哪个项目、
/// 哪个组织、哪个托管平台，「仓库」除了告诉你它是个链接以外什么都没说。
fn repo_path(url: &str) -> &str {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
}

/// 带计数的 chip —— 计数是「值不值得点进去」的唯一信息。
fn chip_count(href: &str, label: &str, n: usize) -> Markup {
    html! {
        a.chip.chip-count href=(href) { (label) span.n { (n) } }
    }
}

/// 卡片语境 —— 决定副标行显示哪一维。
///
/// 不要花一行字告诉读者他刚刚选的那一维：分类页上每张卡都挂着当前分类，
/// vendor 页上每张卡都挂着当前 vendor，两者都是自指控件。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CardContext {
    /// 首页全量列表：显示「替代哪些 SaaS」。
    All,
    /// 分类页：分类已知，显示「替代哪些 SaaS」。
    Category,
    /// vendor 页：vendor 已知，显示信号分类。
    Vendor,
}

/// 工具卡片（首页 / 分类页 / vendor 页复用）。整张卡片是一个链接，内部无链接。
pub fn tool_card(tool: &Tool, catalog: &Catalog, ctx: CardContext) -> Markup {
    // FieldNote 状态优先；没有 FieldNote 时，上游非活跃的话给一个提示 badge ——
    // 目录把已归档项目摆成普通条目就是在误导读者。
    let status_badge = match (&tool.field_note, &tool.upstream) {
        (Some(f), _) => Some(html! {
            span.badge.badge-status data-status=(status_str(f.status)) { (status_str(f.status)) }
        }),
        (None, Some(u)) if u.status != UpstreamStatus::Active => Some(html! {
            span.badge.badge-upstream data-upstream=(upstream_status_str(u.status)) {
                (upstream_label(u.status))
            }
        }),
        _ => None,
    };
    let meta = match ctx {
        CardContext::All | CardContext::Category => (!tool.replaces.is_empty()).then(|| {
            let names: Vec<String> = tool
                .replaces
                .iter()
                .map(|v| vendor_label(catalog, v))
                .collect();
            format!("替代 {}", names.join("、"))
        }),
        CardContext::Vendor => (!tool.categories.is_empty()).then(|| {
            let names: Vec<String> = tool
                .categories
                .iter()
                .map(|c| category_label(catalog, c))
                .collect();
            names.join("、")
        }),
    };
    html! {
        a.card href=(format!("/tools/{}", tool.slug)) {
            .card-head {
                h3 { (tool.name) }
                (status_badge.unwrap_or_default())
            }
            p.card-summary { (tool.summary) }
            @if let Some(m) = meta {
                p.card-meta { (m) }
            }
        }
    }
}

/// 工具网格 —— 空集合渲染成一句话，而不是一个空网格。
fn tool_grid(tools: &[&Tool], catalog: &Catalog, ctx: CardContext) -> Markup {
    html! {
        @if tools.is_empty() {
            p.empty { "暂无条目。" }
        } @else {
            .tool-grid {
                @for t in tools {
                    (tool_card(t, catalog, ctx))
                }
            }
        }
    }
}

fn category_label(catalog: &Catalog, id: &crate::model::CategoryId) -> String {
    catalog
        .category(id)
        .map(|c| c.name.clone())
        .unwrap_or_else(|| id.to_string())
}

/// vendor 展示名 —— 页面上一律用 `vendors.toml` 里的名字，不露裸 id。
fn vendor_label(catalog: &Catalog, id: &crate::model::VendorId) -> String {
    catalog
        .vendor(id)
        .map(|v| v.name.clone())
        .unwrap_or_else(|| id.to_string())
}

fn upstream_status_str(s: UpstreamStatus) -> &'static str {
    match s {
        UpstreamStatus::Active => "Active",
        UpstreamStatus::Maintenance => "Maintenance",
        UpstreamStatus::Archived => "Archived",
    }
}

fn upstream_label(s: UpstreamStatus) -> &'static str {
    match s {
        UpstreamStatus::Active => "上游活跃",
        UpstreamStatus::Maintenance => "上游停滞",
        UpstreamStatus::Archived => "上游已归档",
    }
}

/// 千分位 —— 65768 → 65,768。四位数以上不加分隔符就没法一眼读出量级。
fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// 从 host + full_name 还原仓库地址 —— 活跃度数字标明出处才有意义。
fn repo_url(r: &RepoStats) -> String {
    let host = match r.host.as_str() {
        "gitlab" => "gitlab.com",
        _ => "github.com",
    };
    format!("https://{host}/{}", r.full_name)
}

/// 上游活跃度 —— 全部来自 `xtask fetch` 生成的 repo.json，条目文件里不存这些数字。
///
/// 抓取日期必须显示：fetch 是 fail-soft 的，数字会悄悄变旧，
/// 陈旧度要摆在读者眼前而不是藏在 CI 里
/// （见 `docs/plans/2026-08-20-data-pipeline.md` 的「陈旧度必须可见」）。
fn repo_stats_block(r: &RepoStats, fetched_at: &crate::model::Date) -> Markup {
    html! {
        section.stats {
            .stats-head {
                h2 { "上游活跃度" }
                span.muted {
                    (fetched_at) " 抓取 · 数据来自 "
                    a href=(repo_url(r)) rel="noreferrer" { (r.full_name) }
                    " (" (r.host) ")"
                }
            }
            .stat-row {
                .stat {
                    span.stat-label { "star" }
                    span.stat-value { (thousands(r.stars)) }
                }
                .stat {
                    span.stat-label { "最近提交" }
                    @if let Some(d) = &r.pushed_at {
                        span.stat-value { (d) }
                    } @else {
                        // 只说得到的事实：空仓库没有提交、或抓取时该字段没拿到。
                        // 数字缺失不等于上游停滞（对照最新版本那栏的处理）。
                        span.stat-value.stat-none { "未知" }
                    }
                }
                @if let Some(tag) = &r.latest_release {
                    .stat {
                        span.stat-label { "最新版本" }
                        span.stat-value { (tag) }
                        @if let Some(d) = &r.released_at {
                            span.stat-sub { (d) }
                        }
                    }
                } @else {
                    // 只说得到的事实：该平台上没有 release 记录。
                    // 不等于上游不发版 —— Graylog / Zabbix 都在自己站点上发布。
                    .stat {
                        span.stat-label { "最新版本" }
                        span.stat-value.stat-none { "平台无 release 记录" }
                    }
                }
            }
            p.stat-foot {
                "数字由构建期抓取，非实时；star 数不同代码托管平台之间不可直接比较。"
            }
        }
    }
}

/// 上游状态提示条。归档 / 停滞是选型时最该先看到的一句话，所以放在正文之前。
fn upstream_banner(u: &Upstream) -> Markup {
    html! {
        aside.upstream data-upstream=(upstream_status_str(u.status)) {
            .upstream-head {
                span.badge.badge-upstream data-upstream=(upstream_status_str(u.status)) {
                    (upstream_label(u.status))
                }
                span.muted { (u.as_of) " 核对" }
            }
            p.upstream-note { (u.note) }
            p.muted { "出处: " a href=(u.source.to_string()) { (u.source) } }
        }
    }
}

fn status_str(s: Status) -> &'static str {
    match s {
        Status::Running => "Running",
        Status::Retired => "Retired",
        Status::Rejected => "Rejected",
        Status::Evaluating => "Evaluating",
    }
}

/// 首页：hero + SaaS 替换索引 + 分类导航 + 全部工具。
///
/// SaaS 索引排在分类之前 —— hero 承诺「按你要替换掉哪个 SaaS 索引」，
/// 那这个索引就得是首屏第一个能点的东西。
pub fn home(catalog: &Catalog) -> Markup {
    let mut vendors: Vec<(&Vendor, usize)> = catalog
        .vendors
        .iter()
        .map(|v| (v, catalog.tools_replacing(&v.id).count()))
        // 零条目的 vendor 不进索引：它的页面是一个空列表，入口等于死控件。
        .filter(|(_, n)| *n > 0)
        .collect();
    vendors.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.name.cmp(&b.0.name)));

    let all: Vec<&Tool> = catalog.tools.iter().collect();

    layout(
        "首页",
        html! {
            section.hero {
                h1 { "自托管可观测性工具目录" }
                p {
                    "按「你要替换掉哪个 SaaS」索引。每条给出许可证、实现语言、部署形态、\
                     运行依赖与上手前要知道的坑，以及回到上游仓库与文档的链接。"
                }
            }
            section id="replaces" {
                h2 { "按 SaaS 替换" }
                .chip-row {
                    @for (v, n) in &vendors {
                        (chip_count(&format!("/replaces/{}", v.id), &v.name, *n))
                    }
                }
            }
            section id="categories" {
                h2 { "按信号分类" }
                .category-grid {
                    @for c in &catalog.categories {
                        (category_card(c, catalog.tools_in_category(&c.id).count()))
                    }
                }
            }
            section id="tools" {
                h2 { "全部工具" (span_count(all.len())) }
                (tool_grid(&all, catalog, CardContext::All))
            }
        },
    )
}

fn span_count(n: usize) -> Markup {
    html! {
        span.h2-count { (n) " 条" }
    }
}

fn category_card(c: &Category, count: usize) -> Markup {
    html! {
        a.cat-card href=(format!("/categories/{}", c.id)) {
            h3 { (c.name) }
            span.cat-count { (count) " 条" }
        }
    }
}

/// 分类页。
pub fn category_page(catalog: &Catalog, id: &crate::model::CategoryId) -> Option<Markup> {
    let cat = catalog.category(id)?;
    let tools: Vec<&Tool> = catalog.tools_in_category(id).collect();
    let others: Vec<(&Category, usize)> = catalog
        .categories
        .iter()
        .filter(|c| &c.id != id)
        .map(|c| (c, catalog.tools_in_category(&c.id).count()))
        .collect();
    Some(layout(
        &format!("{} — 分类", cat.name),
        html! {
            (crumb(&[]))
            section.headline {
                h1 { (cat.name) (span_count(tools.len())) }
            }
            (tool_grid(&tools, catalog, CardContext::Category))
            @if !others.is_empty() {
                section.switcher {
                    h2 { "换个分类" }
                    .chip-row {
                        @for (c, n) in &others {
                            (chip_count(&format!("/categories/{}", c.id), &c.name, *n))
                        }
                    }
                }
            }
        },
    ))
}

/// vendor 页：替换某 SaaS 的工具。
pub fn vendor_page(catalog: &Catalog, id: &crate::model::VendorId) -> Option<Markup> {
    let vendor = catalog.vendor(id)?;
    let tools: Vec<&Tool> = catalog.tools_replacing(id).collect();
    Some(layout(
        &format!("{} — 替代方案", vendor.name),
        html! {
            (crumb(&[]))
            section.headline {
                h1 { "替换 " (vendor.name) (span_count(tools.len())) }
            }
            (tool_grid(&tools, catalog, CardContext::Vendor))
        },
    ))
}

/// 工具详情页。
pub fn tool_page(catalog: &Catalog, slug: &crate::model::Slug) -> Option<Markup> {
    let tool = catalog.tool(slug)?;
    let field_note = tool.field_note.as_ref();

    // 面包屑走第一个分类 —— 读者多半就是从那儿点进来的。
    let parents: Vec<(String, String)> = tool
        .categories
        .first()
        .map(|c| vec![(format!("/categories/{}", c), category_label(catalog, c))])
        .unwrap_or_default();

    // 同类工具：把详情页从死胡同改回目录的一个节点。
    // 不截断 —— 最大的分类 17 条，全列出来也就一两行 chip，
    // 而「显示前 N 条」而不声明，读起来就是「一共就这些」。
    let siblings: Vec<&Tool> = tool
        .categories
        .first()
        .map(|cid| {
            catalog
                .tools_in_category(cid)
                .filter(|t| t.slug != tool.slug)
                .collect()
        })
        .unwrap_or_default();

    let field_note_section = field_note.map(|f| {
        let footprint = f.footprint.as_ref().map(|fp| {
            html! {
                .footprint {
                    h3 { "实测足迹" }
                    ul {
                        @if let Some(m) = &fp.mem_peak_7d { li { "内存 7d 峰值: " (m) } }
                        @if let Some(m) = &fp.mem_limit { li { "内存 limit: " (m) } }
                        @if let Some(n) = &fp.note { li.note { (n) } }
                    }
                }
            }
        });
        let gotchas = if f.gotchas.is_empty() {
            None
        } else {
            Some(html! {
                .gotchas {
                    h3 { "踩过的坑" }
                    ul {
                        @for g in &f.gotchas {
                            li {
                                (g.text)
                                @if let Some(url) = &g.evidence {
                                    span { " " }
                                    a href=(url.to_string()) { "[出处]" }
                                }
                            }
                        }
                    }
                }
            })
        };
        let blind = if f.blind_spots.is_empty() {
            None
        } else {
            Some(html! {
                .blind-spots {
                    h3 { "它看不见什么" }
                    ul {
                        @for b in &f.blind_spots { li { (b) } }
                    }
                }
            })
        };
        let clusters = if f.clusters.is_empty() {
            None
        } else {
            Some(html! {
                .clusters {
                    h3 { "运行集群" }
                    ul {
                        @for c in &f.clusters {
                            li {
                                @match catalog.clusters.iter().find(|x| &x.id == c) {
                                    Some(cl) => (cl.name),
                                    None => (c.to_string()),
                                }
                            }
                        }
                    }
                }
            })
        };
        let supersedes = f.supersedes.as_ref().map(|s| {
            let name = catalog
                .tool(s)
                .map(|t| t.name.clone())
                .unwrap_or_else(|| s.to_string());
            html! {
                .supersedes {
                    h3 { "接替者" }
                    p { "退役后由 " a href=(format!("/tools/{}", s)) { (name) } " 接替。" }
                }
            }
        });
        html! {
            section.field-note data-status=(status_str(f.status)) {
                header.field-note-head {
                    h2 { "FieldNote" }
                    span.badge.badge-status data-status=(status_str(f.status)) { (status_str(f.status)) }
                    @if let Some(d) = &f.since { span.muted { "since " (d) } }
                }
                (footprint.unwrap_or_default())
                (gotchas.unwrap_or_default())
                (blind.unwrap_or_default())
                (clusters.unwrap_or_default())
                (supersedes.unwrap_or_default())
                p.decision {
                    "出处: " a href=(f.decision.to_string()) { (f.decision) }
                }
            }
        }
    });

    let deploy = tool
        .self_host
        .iter()
        .map(|d| deploy_kind_str(d))
        .collect::<Vec<_>>()
        .join(" · ");

    Some(layout(
        &format!("{} — 工具", tool.name),
        html! {
            (crumb(&parents))
            article.tool-page {
                h1 { (tool.name) }
                p.lead { (tool.summary) }
                @if let Some(u) = &tool.upstream {
                    (upstream_banner(u))
                }
                section.detail {
                    @for para in tool.detail.split("\n\n") {
                        p { (para) }
                    }
                }
                dl.meta {
                    dt { "License" }
                    dd { (tool.license) }
                    dt { "语言" }
                    dd { (tool.language) }
                    @if !deploy.is_empty() {
                        dt { "自托管" }
                        dd { (deploy) }
                    }
                    dt { "上游托管版" }
                    dd { @if tool.hosted { "有" } @else { "无" } }
                    @if !tool.categories.is_empty() {
                        dt { "分类" }
                        dd.dd-chips {
                            @for c in &tool.categories {
                                (chip(&format!("/categories/{}", c), &category_label(catalog, c)))
                            }
                        }
                    }
                    @if !tool.replaces.is_empty() {
                        dt { "可替换" }
                        dd.dd-chips {
                            @for v in &tool.replaces {
                                (chip(&format!("/replaces/{}", v), &vendor_label(catalog, v)))
                            }
                        }
                    }
                    dt { "仓库" }
                    dd {
                        a.repo-link href=(tool.links.repo.to_string()) rel="noreferrer" {
                            (repo_path(tool.links.repo.as_str()))
                            span.ext-mark aria-hidden="true" { "↗" }
                        }
                    }
                    @if tool.links.docs.is_some() || tool.links.site.is_some() {
                        dt { "上游文档" }
                        dd.dd-chips {
                            @if let Some(d) = &tool.links.docs {
                                (chip_ext(&d.to_string(), "文档"))
                            }
                            @if let Some(si) = &tool.links.site {
                                (chip_ext(&si.to_string(), "官网"))
                            }
                        }
                    }
                }
                @if let (Some(r), Some(idx)) = (catalog.repo_stats(&tool.slug), &catalog.repos) {
                    (repo_stats_block(r, &idx.fetched_at))
                }
                @if !tool.requires.is_empty() {
                    section.facts {
                        h2 { "运行依赖" }
                        ul {
                            @for r in &tool.requires { li { (r) } }
                        }
                    }
                }
                @if !tool.watch.is_empty() {
                    section.facts.facts-watch {
                        h2 { "上手前注意" }
                        ul {
                            @for w in &tool.watch { li { (w) } }
                        }
                    }
                }
                @match &field_note_section {
                    Some(s) => (s),
                    // 覆盖率不均是诚实的 —— 明说「这条只是元数据」，
                    // 而不是让读者对着一个空白处猜证据在哪。
                    // 只说得到的事实：这条还没有 FieldNote。**不要**替作者断言
                    // 「没跑过」—— 段 3 才落库，没落库不等于没跑过。
                    None => p.no-field-note {
                        "暂无 FieldNote —— 这条目前是元数据条目。"
                    },
                }
                @if !siblings.is_empty() {
                    section.siblings {
                        h2 { "同类工具（" (category_label(catalog, &tool.categories[0])) "）" }
                        .chip-row {
                            @for t in &siblings {
                                (chip(&format!("/tools/{}", t.slug), &t.name))
                            }
                        }
                    }
                }
            }
        },
    ))
}

fn deploy_kind_str(d: &crate::model::DeployKind) -> &'static str {
    use crate::model::DeployKind::*;
    match d {
        SingleBinary => "单二进制",
        Docker => "Docker",
        Compose => "Docker Compose",
        HelmChart => "Helm Chart",
        Operator => "K8s Operator",
        DebRpm => "deb / rpm",
        SourceBuild => "源码构建",
    }
}

/// 404 页。
pub fn not_found() -> Markup {
    layout(
        "404",
        html! {
            section.not-found {
                h1 { "404" }
                p { "没有这个页面。" }
                a href="/" { "← 回目录" }
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Catalog, Category, Cluster, DeployKind, FieldNote, Footprint, Gotcha, Links, Slug, Spdx,
        Status, Summary, Tool, Upstream, UpstreamStatus, Url, Vendor,
    };

    /// 只取 `</style>` 之后的部分。CSS 是内联进每个页面的，选择器与注释里
    /// 都会出现和正文一样的字符串 —— 断言「页面上没有 X」必须先把 CSS 切掉。
    fn body_of(html: &str) -> &str {
        html.split_once("</style>").map(|(_, b)| b).unwrap_or(html)
    }

    fn tool(slug: &str, name: &str, field_note: Option<FieldNote>) -> Tool {
        Tool {
            slug: Slug::new(slug),
            name: name.into(),
            summary: Summary::new("CNCF 毕业的指标事实标准。"),
            license: Spdx::new("Apache-2.0"),
            language: "Go".into(),
            categories: vec![crate::model::CategoryId::new("metrics")],
            replaces: vec![crate::model::VendorId::new("datadog")],
            self_host: vec![DeployKind::SingleBinary, DeployKind::HelmChart],
            hosted: false,
            links: Links {
                repo: Url::new("https://github.com/prometheus/prometheus"),
                site: Some(Url::new("https://prometheus.io/")),
                docs: Some(Url::new("https://prometheus.io/docs/")),
            },
            detail: "拉模型 + PromQL + 本地 TSDB。".into(),
            requires: vec!["对象存储".into()],
            watch: vec!["单机 TSDB 没有副本概念".into()],
            upstream: None,
            field_note,
        }
    }

    fn field_note() -> FieldNote {
        FieldNote {
            status: Status::Running,
            since: Some(crate::model::Date::new("2026-03-01")),
            clusters: vec![crate::model::ClusterId::new("homelab")],
            footprint: Some(Footprint {
                mem_peak_7d: Some("2715Mi".into()),
                mem_limit: Some("3072Mi".into()),
                note: Some("12GB VM 上最大的单一内存消耗者".into()),
            }),
            gotchas: vec![Gotcha {
                text: "kubelet 的 /metrics 把 legacyregistry 全吐出来".into(),
                evidence: Some(Url::new(
                    "https://example.com/homelab/records/prometheus.md",
                )),
            }],
            blind_spots: vec!["remote-write 断链时中枢看不见 oracle 侧".into()],
            supersedes: None,
            decision: Url::new("https://example.com/homelab/decisions/prometheus.md"),
        }
    }

    fn catalog_with_fieldnote() -> Catalog {
        Catalog {
            vendors: vec![
                Vendor {
                    id: crate::model::VendorId::new("datadog"),
                    name: "Datadog".into(),
                },
                // 零覆盖 vendor —— 不该出现在首页索引里。
                Vendor {
                    id: crate::model::VendorId::new("logz-io"),
                    name: "Logz.io".into(),
                },
            ],
            categories: vec![Category {
                id: crate::model::CategoryId::new("metrics"),
                name: "指标".into(),
            }],
            clusters: vec![Cluster {
                id: crate::model::ClusterId::new("homelab"),
                name: "homelab (k3s)".into(),
            }],
            tools: vec![tool("prometheus", "Prometheus", Some(field_note()))],
            repos: None,
        }
    }

    #[test]
    fn tool_page_renders_field_note() {
        let catalog = catalog_with_fieldnote();
        let html = tool_page(&catalog, &Slug::new("prometheus"))
            .expect("工具应存在")
            .into_string();
        assert!(html.contains("FieldNote"));
        assert!(html.contains("Running"));
        assert!(html.contains("2715Mi"));
        assert!(html.contains("踩过的坑"));
        assert!(html.contains("它看不见什么"));
        assert!(html.contains("出处"));
        // 集群显示名，不是裸 id。
        assert!(html.contains("homelab (k3s)"));
    }

    #[test]
    fn tool_page_renders_404_for_missing() {
        let catalog = catalog_with_fieldnote();
        assert!(tool_page(&catalog, &Slug::new("nope")).is_none());
    }

    #[test]
    fn home_renders_categories_and_tools() {
        let catalog = catalog_with_fieldnote();
        let html = home(&catalog).into_string();
        assert!(html.contains("指标"));
        assert!(html.contains("Prometheus"));
    }

    /// 卡片整体是一个 `<a>`；内部再放 `<a>` 是非法 HTML —— 浏览器会把内层
    /// 拆成兄弟节点，那一行 tag 变成独立的网格单元（40 张卡曾渲染出 80 个格子）。
    #[test]
    fn cards_have_no_nested_anchor() {
        let catalog = catalog_with_fieldnote();
        for html in [
            home(&catalog).into_string(),
            category_page(&catalog, &crate::model::CategoryId::new("metrics"))
                .unwrap()
                .into_string(),
            vendor_page(&catalog, &crate::model::VendorId::new("datadog"))
                .unwrap()
                .into_string(),
        ] {
            let mut rest = html.as_str();
            while let Some(i) = rest.find("<a class=\"card\"") {
                let open = &rest[i + 2..];
                let end = open.find("</a>").expect("卡片链接应闭合");
                assert!(
                    !open[..end].contains("<a "),
                    "卡片内部出现了嵌套链接：{}",
                    &open[..end.min(200)]
                );
                rest = &open[end..];
            }
        }
    }

    /// 每个工具页都必须有一个指向 `links.repo` 的链接，且链接文字是仓库路径
    /// 而不是「仓库」这种没有信息量的词。
    #[test]
    fn every_tool_page_links_to_its_repo() {
        let catalog = catalog_with_fieldnote();
        for t in &catalog.tools {
            let html = tool_page(&catalog, &t.slug)
                .expect("工具应存在")
                .into_string();
            let body = body_of(&html);
            assert!(
                body.contains(&format!("href=\"{}\"", t.links.repo)),
                "{} 页面上没有指向 links.repo 的链接",
                t.slug
            );
            assert!(
                body.contains(repo_path(t.links.repo.as_str())),
                "{} 的仓库链接应该显示路径本身",
                t.slug
            );
        }
    }

    #[test]
    fn tool_page_uses_vendor_display_name_not_id() {
        let catalog = catalog_with_fieldnote();
        let html = tool_page(&catalog, &Slug::new("prometheus"))
            .expect("工具应存在")
            .into_string();
        assert!(
            html.contains(">Datadog</a>"),
            "可替换应显示 vendors.toml 的展示名"
        );
    }

    #[test]
    fn home_indexes_vendors_but_skips_empty_ones() {
        let catalog = catalog_with_fieldnote();
        let html = home(&catalog).into_string();
        assert!(html.contains("/replaces/datadog"), "首页应有 SaaS 替换索引");
        assert!(
            !html.contains("/replaces/logz-io"),
            "零条目的 vendor 不该进索引（它的页面是空列表）"
        );
    }

    #[test]
    fn tool_page_renders_detail_requires_watch_and_upstream_links() {
        let mut catalog = catalog_with_fieldnote();
        catalog.tools = vec![tool("loki", "Loki", None)];
        let html = tool_page(&catalog, &Slug::new("loki"))
            .expect("工具应存在")
            .into_string();
        assert!(html.contains("拉模型 + PromQL"), "技术介绍应渲染");
        assert!(html.contains("运行依赖"));
        assert!(html.contains("上手前注意"));
        assert!(
            html.contains("https://github.com/prometheus/prometheus"),
            "上游仓库链接是条目的出处，必须出现在页面上"
        );
    }

    /// 已归档的上游必须在卡片与详情页上都能看出来 ——
    /// 目录把归档项目摆成普通条目就是在误导读者。
    #[test]
    fn archived_upstream_is_flagged_on_card_and_page() {
        let mut catalog = catalog_with_fieldnote();
        let mut t = tool("grafana-oncall", "Grafana OnCall", None);
        t.upstream = Some(Upstream {
            status: UpstreamStatus::Archived,
            as_of: crate::model::Date::new("2026-03-24"),
            note: "OSS 版仓库已归档。".into(),
            source: Url::new("https://grafana.com/docs/oncall/latest/set-up/open-source/"),
        });
        catalog.tools = vec![t];
        let page = tool_page(&catalog, &Slug::new("grafana-oncall"))
            .expect("工具应存在")
            .into_string();
        assert!(page.contains("上游已归档"));
        assert!(page.contains("2026-03-24"));
        let home_html = home(&catalog).into_string();
        assert!(
            home_html.contains("上游已归档"),
            "首页卡片也要标出来，不能只在详情页说"
        );
    }

    /// 页面不该出现没有内容支撑的承诺：0 条 FieldNote 时不能宣称「一手运维证据」。
    #[test]
    fn pages_make_no_unbacked_first_hand_evidence_claim() {
        let mut catalog = catalog_with_fieldnote();
        catalog.tools = vec![tool("loki", "Loki", None)];
        for html in [
            home(&catalog).into_string(),
            tool_page(&catalog, &Slug::new("loki"))
                .unwrap()
                .into_string(),
        ] {
            let body = body_of(&html);
            assert!(
                !body.contains("一手运维证据"),
                "承诺没有内容兜底就不该写在页面上"
            );
            assert!(!body.contains("每个数字都能追回"));
        }
    }

    /// 活跃度数字必须来自生成数据，且抓取日期要显示在页面上。
    #[test]
    fn repo_stats_render_with_fetch_date() {
        use crate::model::{RepoIndex, RepoStats};
        let mut catalog = catalog_with_fieldnote();
        catalog.tools = vec![tool("prometheus", "Prometheus", None)];
        let mut repos = std::collections::BTreeMap::new();
        repos.insert(
            "prometheus".to_string(),
            RepoStats {
                host: "github".into(),
                full_name: "prometheus/prometheus".into(),
                stars: 65768,
                pushed_at: Some(crate::model::Date::new("2026-08-21")),
                latest_release: Some("v3.14.0".into()),
                released_at: Some(crate::model::Date::new("2026-08-18")),
                license: Some("Apache-2.0".into()),
            },
        );
        catalog.repos = Some(RepoIndex {
            fetched_at: crate::model::Date::new("2026-08-21"),
            repos,
        });
        let html = tool_page(&catalog, &Slug::new("prometheus"))
            .expect("工具应存在")
            .into_string();
        assert!(html.contains("65,768"), "star 数要带千分位");
        assert!(html.contains("v3.14.0"));
        assert!(
            html.contains("2026-08-21 抓取"),
            "抓取日期必须显示 —— fail-soft 的数字会悄悄变旧"
        );
    }

    /// 没有生成数据时页面照常渲染（fetch 从没跑过也是正常状态）。
    #[test]
    fn missing_repo_stats_degrades_quietly() {
        let mut catalog = catalog_with_fieldnote();
        catalog.tools = vec![tool("loki", "Loki", None)];
        catalog.repos = None;
        let html = tool_page(&catalog, &Slug::new("loki"))
            .expect("工具应存在")
            .into_string();
        let body = body_of(&html);
        assert!(!body.contains("stats-head"));
        assert!(!body.contains("stat-value"));
        assert!(body.contains("Loki"));
    }

    #[test]
    fn tool_page_without_field_note_says_so() {
        let mut catalog = catalog_with_fieldnote();
        catalog.tools = vec![tool("loki", "Loki", None)];
        let html = tool_page(&catalog, &Slug::new("loki"))
            .expect("工具应存在")
            .into_string();
        assert!(
            html.contains("元数据条目"),
            "没有 FieldNote 时应明说，而不是留白"
        );
        assert!(!html.contains("FieldNote</h2>"));
    }

    #[test]
    fn empty_vendor_page_says_so_instead_of_blank_grid() {
        let catalog = catalog_with_fieldnote();
        let html = vendor_page(&catalog, &crate::model::VendorId::new("logz-io"))
            .expect("vendor 应存在")
            .into_string();
        assert!(html.contains("暂无条目"));
    }
}
