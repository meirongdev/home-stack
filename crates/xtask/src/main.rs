//! `crates/xtask` —— 构建期工具（native）。
//!
//! 已实现 `validate`（内容引用完整性，硬失败）与 `fetch`（上游活跃度，fail-soft）。
//! `render-diff`（段 2）按 ROADMAP 补齐；
//! `--help` 里没有的子命令，pre-push 钩子会自动跳过。

use std::path::Path;

const SUBCOMMANDS: &[(&str, &str)] = &[
    (
        "validate",
        "内容引用完整性校验（四类硬失败：引用未声明 / 孤儿分类 / slug 重复 / summary 超长）",
    ),
    (
        "fetch",
        "抓上游活跃度 → content/generated/repo.json（fail-soft，需 GITHUB_TOKEN）",
    ),
];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str);

    match cmd {
        Some("validate") => validate(),
        Some("fetch") => fetch(),
        Some("--help") | Some("-h") | None => {
            print_help();
            std::process::exit(0);
        }
        Some(other) => {
            eprintln!("未知子命令: {other}");
            print_help();
            std::process::exit(2);
        }
    }
}

fn print_help() {
    println!("xtask —— 构建期工具");
    println!();
    println!("用法: cargo run -p xtask -- <子命令>");
    println!();
    println!("子命令:");
    for (name, desc) in SUBCOMMANDS {
        println!("  {name:<12} {desc}");
    }
}

/// content 目录：优先 `XTASK_CONTENT_DIR`（测试与 CI 用），否则仓库内相对路径。
fn content_dir() -> std::path::PathBuf {
    match std::env::var_os("XTASK_CONTENT_DIR") {
        Some(d) => Path::new(&d).to_path_buf(),
        None => Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
    }
}

/// 生成数据的陈旧度阈值。fail-soft 的代价是数字会悄悄变旧，
/// 所以 validate 每次都吵一声 —— 规格里点名不要依赖「没人看的 CI 徽章」。
const REPO_JSON_STALE_DAYS: i64 = 7;

fn validate() {
    let content = content_dir();
    let sources = read_sources(&content);
    let refs: Vec<(&str, &str)> = sources
        .iter()
        .map(|(p, r)| (p.as_str(), r.as_str()))
        .collect();
    warn_if_repo_json_stale(&content);
    match site::load::load(&refs) {
        Ok(catalog) => {
            println!(
                "✓ validate 通过：{} 个工具、{} 个分类、{} 个 vendor、{} 个集群",
                catalog.tools.len(),
                catalog.categories.len(),
                catalog.vendors.len(),
                catalog.clusters.len(),
            );
        }
        Err(issues) => {
            for i in &issues {
                print_issue(i);
            }
            eprintln!("\nvalidate 未通过：{} 处问题", issues.len());
            std::process::exit(1);
        }
    }
}

fn print_issue(i: &site::load::Issue) {
    println!("error: {}", i.message);
    println!("  --> {}:{}:{}", i.path, i.line, i.col);
    if let Some(src) = &i.source_line {
        println!("   |");
        println!(" {:>4} | {}", i.line, src);
        let pad = " ".repeat(i.col - 1);
        let carets = "^".repeat((i.end_col.saturating_sub(i.col)).max(1));
        println!("   | {pad}{carets}");
    }
    println!("   |");
    if let Some(h) = &i.help {
        println!("   = help: {h}");
    }
    if let Some(n) = &i.note {
        println!("   = note: {n}");
    }
    println!();
}

fn read_sources(dir: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("无法读取 {}: {e}", dir.display()))
        .filter_map(Result::ok)
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            let raw = std::fs::read_to_string(&path).expect("读取失败");
            out.push((path.to_string_lossy().into_owned(), raw));
        }
    }
    // 生成数据（xtask fetch 产物）。缺失是正常状态 —— 页面退化成不显示活跃度。
    let generated = dir.join("generated/repo.json");
    if generated.is_file() {
        if let Ok(raw) = std::fs::read_to_string(&generated) {
            out.push((generated.to_string_lossy().into_owned(), raw));
        }
    }

    let tools_dir = dir.join("tools");
    if tools_dir.is_dir() {
        let mut tools: Vec<_> = std::fs::read_dir(&tools_dir)
            .unwrap_or_else(|e| panic!("无法读取 {}: {e}", tools_dir.display()))
            .filter_map(Result::ok)
            .collect();
        tools.sort_by_key(|e| e.file_name());
        for entry in tools {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                let raw = std::fs::read_to_string(&path).expect("读取失败");
                out.push((path.to_string_lossy().into_owned(), raw));
            }
        }
    }
    out
}

// ═══════════════════════════════════════════════════════════════════════
// fetch —— 构建期抓上游活跃度，产出 content/generated/repo.json
//
// 规格：docs/plans/2026-08-20-data-pipeline.md
//   · stars / pushedAt / latestRelease / licenseInfo 一律**生成**，不手写 ——
//     会过期的数字不进条目文件。
//   · **必须 fail-soft**：抓不到就保留上一次 committed 的产物、只 warning、退出码 0。
//     一次上游抖动不该让站点构建失败（对照：validate 必须硬失败）。
//   · 产物 committed 进仓库，git 历史顺带成为「这些数字什么时候变的」的记录。
//
// HTTP 走 `curl` 子进程而不是 reqwest / ureq：构建期工具不需要异步栈，
// 而少一条依赖就少一份 wasm 无关的编译时间。要换成 ureq 只需替换 `http_post`
// 与 `http_get` 两个函数，其余逻辑不动。
// ═══════════════════════════════════════════════════════════════════════

/// 一个抓取目标：由条目里的 `links.repo` 解析出来，不额外维护映射表。
#[derive(Debug)]
struct Target {
    slug: String,
    host: Host,
    owner: String,
    name: String,
    /// 条目里手写的许可证，用来和上游检测结果交叉核对。
    declared_license: String,
}

#[derive(Debug, PartialEq, Eq)]
enum Host {
    GitHub,
    GitLab,
}

#[derive(serde::Deserialize)]
struct ToolHead {
    slug: String,
    license: String,
    links: ToolLinks,
}

#[derive(serde::Deserialize)]
struct ToolLinks {
    repo: String,
}

fn fetch() {
    let content = content_dir();
    let out_path = content.join("generated/repo.json");

    let targets = collect_targets(&content);
    if targets.is_empty() {
        eprintln!("warning: 没有找到任何带 links.repo 的条目，fetch 跳过");
        return;
    }
    println!("fetch: {} 个条目", targets.len());

    // 上一次的产物 —— fail-soft 的兜底来源，逐条回退而不是整份丢弃。
    let previous: serde_json::Value = std::fs::read_to_string(&out_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({"repos": {}}));

    let mut repos = serde_json::Map::new();
    let mut failed: Vec<&str> = Vec::new();

    // ── GitHub：一次 GraphQL 拿全部，避免 40 次 REST 打配额 ──────────────
    let gh: Vec<&Target> = targets.iter().filter(|t| t.host == Host::GitHub).collect();
    if !gh.is_empty() {
        match fetch_github(&gh) {
            Ok(map) => {
                for (slug, v) in map {
                    repos.insert(slug, v);
                }
            }
            Err(e) => eprintln!("warning: GitHub 抓取失败（{e}），这批将回退到上一次的产物"),
        }
    }

    // ── GitLab：条目少，逐个 REST ────────────────────────────────────────
    for t in targets.iter().filter(|t| t.host == Host::GitLab) {
        match fetch_gitlab(t) {
            Ok(v) => {
                repos.insert(t.slug.clone(), v);
            }
            Err(e) => eprintln!("warning: GitLab {}/{} 抓取失败（{e}）", t.owner, t.name),
        }
    }

    // 逐条 fail-soft：这次没拿到的，沿用上一次 committed 的值。
    for t in &targets {
        if repos.contains_key(&t.slug) {
            continue;
        }
        failed.push(&t.slug);
        if let Some(old) = previous
            .get("repos")
            .and_then(|r| r.get(&t.slug))
            .filter(|v| !v.is_null())
        {
            repos.insert(t.slug.clone(), old.clone());
        }
    }

    if !failed.is_empty() {
        eprintln!(
            "warning: {} 条没抓到（{}）—— 已沿用上一次的值。fetch 是 fail-soft，退出码仍为 0",
            failed.len(),
            failed.join(", ")
        );
    }

    license_cross_check(&targets, &repos);

    let doc = serde_json::json!({
        "fetched_at": today(),
        "repos": repos,
    });
    if let Some(parent) = out_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let body = serde_json::to_string_pretty(&doc).expect("序列化失败");
    std::fs::write(&out_path, body + "\n").expect("写入 repo.json 失败");
    println!(
        "✓ fetch 完成：{} 条写入 {}（fetched_at = {}）",
        repos.len(),
        out_path.display(),
        today()
    );
}

/// 从条目的 `links.repo` 解析抓取目标 —— 不额外维护 slug→repo 映射表，
/// 那种表一定会和内容漂移。
fn collect_targets(content: &Path) -> Vec<Target> {
    let dir = content.join("tools");
    let mut entries: Vec<_> = match std::fs::read_dir(&dir) {
        Ok(rd) => rd.filter_map(Result::ok).collect(),
        Err(e) => {
            eprintln!("warning: 无法读取 {}: {e}", dir.display());
            return Vec::new();
        }
    };
    entries.sort_by_key(|e| e.file_name());
    let mut out = Vec::new();
    for e in entries {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("toml") {
            continue;
        }
        let raw = match std::fs::read_to_string(&p) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let head: ToolHead = match toml::from_str(&raw) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("warning: {} 解析失败：{e}", p.display());
                continue;
            }
        };
        match parse_repo_url(&head.links.repo) {
            Some((host, owner, name)) => out.push(Target {
                slug: head.slug,
                host,
                owner,
                name,
                declared_license: head.license,
            }),
            None => eprintln!(
                "warning: {} 的 links.repo 不是可识别的 GitHub / GitLab 地址：{}",
                head.slug, head.links.repo
            ),
        }
    }
    out
}

/// `https://github.com/owner/name` → (GitHub, owner, name)。
fn parse_repo_url(url: &str) -> Option<(Host, String, String)> {
    let rest = url.strip_prefix("https://")?;
    let (host, path) = rest.split_once('/')?;
    let host = match host {
        "github.com" => Host::GitHub,
        "gitlab.com" => Host::GitLab,
        _ => return None,
    };
    let mut parts = path.trim_end_matches('/').split('/');
    let owner = parts.next()?.to_string();
    let name = parts.next()?.trim_end_matches(".git").to_string();
    if owner.is_empty() || name.is_empty() {
        return None;
    }
    Some((host, owner, name))
}

fn fetch_github(targets: &[&Target]) -> Result<Vec<(String, serde_json::Value)>, String> {
    let token = std::env::var("GITHUB_TOKEN")
        .or_else(|_| std::env::var("GH_TOKEN"))
        .map_err(|_| "缺少 GITHUB_TOKEN / GH_TOKEN（GraphQL 必须认证）".to_string())?;

    // 别名批量查询：一次请求拿全部仓库的四个字段。
    let mut q = String::from("query {");
    for (i, t) in targets.iter().enumerate() {
        q.push_str(&format!(
            r#" r{i}: repository(owner: "{}", name: "{}") {{ stargazerCount pushedAt licenseInfo {{ spdxId }} latestRelease {{ tagName publishedAt }} }}"#,
            t.owner, t.name
        ));
    }
    q.push('}');

    let body = serde_json::json!({ "query": q }).to_string();
    let raw = http_post("https://api.github.com/graphql", &token, &body)?;
    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    if let Some(errs) = v.get("errors") {
        eprintln!("warning: GitHub GraphQL 返回了 errors：{errs}");
    }
    let data = v.get("data").ok_or("响应里没有 data 字段")?;

    let mut out = Vec::new();
    for (i, t) in targets.iter().enumerate() {
        let node = match data.get(format!("r{i}")) {
            Some(n) if !n.is_null() => n,
            _ => continue, // 单条拿不到就交给外层逐条 fail-soft
        };
        out.push((
            t.slug.clone(),
            serde_json::json!({
                "host": "github",
                "full_name": format!("{}/{}", t.owner, t.name),
                "stars": node.get("stargazerCount").and_then(|x| x.as_u64()).unwrap_or(0),
                "pushed_at": date_only(node.get("pushedAt")),
                "latest_release": node.pointer("/latestRelease/tagName"),
                "released_at": date_only(node.pointer("/latestRelease/publishedAt")),
                "license": node.pointer("/licenseInfo/spdxId")
                    .and_then(|x| x.as_str())
                    // GitHub 检测不出 FSL / SSPL / 拆分许可时给 NOASSERTION，
                    // 那是「没结论」不是「许可证是这个」，不该写进产物。
                    .filter(|s| *s != "NOASSERTION"),
            }),
        ));
    }
    Ok(out)
}

fn fetch_gitlab(t: &Target) -> Result<serde_json::Value, String> {
    let id = format!("{}%2F{}", t.owner, t.name);
    let proj: serde_json::Value = serde_json::from_str(&http_get(&format!(
        "https://gitlab.com/api/v4/projects/{id}?license=true"
    ))?)
    .map_err(|e| e.to_string())?;
    let rel: serde_json::Value = serde_json::from_str(
        &http_get(&format!(
            "https://gitlab.com/api/v4/projects/{id}/releases?per_page=1"
        ))
        .unwrap_or_else(|_| "[]".into()),
    )
    .unwrap_or(serde_json::Value::Array(vec![]));
    let latest = rel.get(0);

    // GitLab 的 `last_activity_at` 包含 issue / MR 活动，不等于最近提交 ——
    // 页面上那一栏写的是「最近提交」，就得取真的提交时间。
    let commits: serde_json::Value = serde_json::from_str(
        &http_get(&format!(
            "https://gitlab.com/api/v4/projects/{id}/repository/commits?per_page=1"
        ))
        .unwrap_or_else(|_| "[]".into()),
    )
    .unwrap_or(serde_json::Value::Array(vec![]));
    let pushed = commits
        .get(0)
        .and_then(|c| c.get("committed_date"))
        .cloned();

    Ok(serde_json::json!({
        "host": "gitlab",
        "full_name": format!("{}/{}", t.owner, t.name),
        "stars": proj.get("star_count").and_then(|x| x.as_u64()).unwrap_or(0),
        "pushed_at": date_only(pushed.as_ref()),
        "latest_release": latest.and_then(|r| r.get("tag_name")).cloned(),
        "released_at": date_only(latest.and_then(|r| r.get("released_at"))),
        "license": proj.pointer("/license/nickname")
            .or_else(|| proj.pointer("/license/name"))
            .cloned(),
    }))
}

/// 手写许可证 vs 上游检测结果。**只 warning**：GitHub 对 FSL / SSPL / 开源核心
/// 拆分许可一律检测不出，硬失败会天天误报。但真不一致时必须吵一声 ——
/// 站点把 license 当硬事实展示，错了是最难被发现的一类错。
fn license_cross_check(targets: &[Target], repos: &serde_json::Map<String, serde_json::Value>) {
    let mut mismatch = 0;
    for t in targets {
        let Some(upstream) = repos
            .get(&t.slug)
            .and_then(|v| v.get("license"))
            .and_then(|v| v.as_str())
        else {
            continue;
        };
        let norm = |s: &str| s.to_ascii_lowercase().replace(['-', '.', ' '], "");
        if !norm(&t.declared_license).starts_with(&norm(upstream))
            && !norm(upstream).starts_with(&norm(&t.declared_license))
        {
            eprintln!(
                "warning: {} 的 license 与上游检测不一致 —— 条目写 `{}`，上游检测 `{}`",
                t.slug, t.declared_license, upstream
            );
            mismatch += 1;
        }
    }
    if mismatch == 0 {
        println!("✓ license 交叉核对：无不一致");
    } else {
        eprintln!("warning: {mismatch} 条 license 需要人工确认（拆分许可 / 上游改授权都会命中）");
    }
}

// ── HTTP：换成 ureq / reqwest 只需替换这两个函数 ────────────────────────

fn http_post(url: &str, token: &str, body: &str) -> Result<String, String> {
    run_curl(&[
        "-sS",
        "--fail-with-body",
        "--max-time",
        "60",
        "-X",
        "POST",
        "-H",
        &format!("Authorization: bearer {token}"),
        "-H",
        "Content-Type: application/json",
        "-H",
        "User-Agent: probe-directory-xtask",
        "--data-binary",
        body,
        url,
    ])
}

fn http_get(url: &str) -> Result<String, String> {
    run_curl(&[
        "-sS",
        "--fail-with-body",
        "--max-time",
        "30",
        "-H",
        "User-Agent: probe-directory-xtask",
        url,
    ])
}

fn run_curl(args: &[&str]) -> Result<String, String> {
    let out = std::process::Command::new("curl")
        .args(args)
        .output()
        .map_err(|e| format!("无法执行 curl：{e}"))?;
    if !out.status.success() {
        return Err(format!(
            "curl 退出码 {:?}：{}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

// ── 日期：不引 chrono，只需要 YYYY-MM-DD ────────────────────────────────

/// ISO 8601 时间戳 → `YYYY-MM-DD`（只保留日期部分）。
fn date_only(v: Option<&serde_json::Value>) -> serde_json::Value {
    match v.and_then(|x| x.as_str()) {
        Some(s) if s.len() >= 10 => serde_json::Value::String(s[..10].to_string()),
        _ => serde_json::Value::Null,
    }
}

/// 今天（UTC），`YYYY-MM-DD`。用 Howard Hinnant 的 civil_from_days，省一条 chrono 依赖。
fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, m, d) = civil_from_days((secs / 86_400) as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// repo.json 超过阈值就在 validate 输出里 warning。
///
/// 不硬失败：抓取管道停摆是运维问题，不是内容错误，拦住构建反而把公网站点
/// 绑回了管道可用性（同 fetch 的 fail-soft 论证）。
fn warn_if_repo_json_stale(content: &Path) {
    let path = content.join("generated/repo.json");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        println!("note: 还没有 content/generated/repo.json —— 页面不显示上游活跃度");
        return;
    };
    let Some(fetched) = serde_json::from_str::<serde_json::Value>(&raw)
        .ok()
        .and_then(|v| v.get("fetched_at")?.as_str().map(str::to_string))
    else {
        eprintln!("warning: repo.json 里没有 fetched_at");
        return;
    };
    match days_since(&fetched) {
        Some(d) if d > REPO_JSON_STALE_DAYS => eprintln!(
            "warning: 上游活跃度数据已 {d} 天未更新（fetched_at = {fetched}，阈值 {REPO_JSON_STALE_DAYS} 天）—— 跑一次 `xtask fetch`"
        ),
        Some(d) => println!("✓ 上游活跃度数据 {d} 天前抓取（fetched_at = {fetched}）"),
        None => eprintln!("warning: repo.json 的 fetched_at 不是 YYYY-MM-DD：{fetched}"),
    }
}

/// `YYYY-MM-DD` 距今天多少天。
fn days_since(date: &str) -> Option<i64> {
    let mut it = date.split('-');
    let y: i64 = it.next()?.parse().ok()?;
    let m: u32 = it.next()?.parse().ok()?;
    let d: u32 = it.next()?.parse().ok()?;
    let then = days_from_civil(y, m, d);
    let now = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs()
        / 86_400) as i64;
    Some(now - then)
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let mp = if m > 2 { m - 3 } else { m + 9 } as u64;
    let doy = (153 * mp + 2) / 5 + d as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe as i64 - 719_468
}
