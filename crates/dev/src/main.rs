//! `crates/dev` —— 本地开发入口（native）。
//!
//! tokio + axum::serve，`cargo run -p dev` → `:8080`。
//! 从 `content/` 读取 TOML，构建并校验 `Catalog`（失败即退出），
//! 然后喂给 `crates/site` 的 Router。

use site::model::Catalog;
use std::path::Path;

fn main() {
    let content_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let catalog = match load_catalog(&content_dir) {
        Ok(c) => c,
        Err(issues) => {
            eprintln!("Catalog 校验未通过，无法启动：");
            for i in &issues {
                eprintln!("  error: {}", i.message);
                if let Some(h) = &i.help {
                    eprintln!("    help: {h}");
                }
            }
            std::process::exit(1);
        }
    };
    let catalog: &'static Catalog = Box::leak(Box::new(catalog));

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("构建 tokio runtime 失败");
    rt.block_on(async move {
        let app = site::router::app().with_state(catalog);
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("绑定 :8080 失败");
        println!("dev server: http://{addr}");
        axum::serve(listener, app).await.expect("serve 失败");
    });
}

fn load_catalog(dir: &Path) -> Result<Catalog, Vec<site::load::Issue>> {
    let sources = read_sources(dir);
    let refs: Vec<(&str, &str)> = sources
        .iter()
        .map(|(p, r)| (p.as_str(), r.as_str()))
        .collect();
    site::load::load(&refs)
}

/// 读取 content 目录下所有 `.toml`，返回 `(path, raw)` 对。
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

    // tools 子目录
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
