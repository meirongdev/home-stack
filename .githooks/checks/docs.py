#!/usr/bin/env python3
"""文档规则检查 —— 机械化 docs/README.md 里的 R2（命名）/ R3（文首必填）+ 链接完整性。

R1（目录归属）无法机械判定，仍靠人守。
全仓库扫描：文档只有十几份，跑一次是毫秒级，不做增量。
"""
import os, re, sys, glob

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
ERR = []
def err(loc, msg, hint=None):
    ERR.append((loc, msg, hint))

# ── 文件分类 ──────────────────────────────────────────────────────────
def classify(rel):
    if rel == "README.md":                       return "root"
    if rel == "docs/README.md":                  return "index"
    if re.fullmatch(r"docs/\w+/README\.md", rel): return "index"
    if rel in ("docs/ARCHITECTURE.md", "docs/ROADMAP.md"): return "living"
    if rel.startswith("docs/decisions/"):        return "adr"
    if rel.startswith("docs/plans/"):            return "plan"
    if rel.startswith("docs/reference/"):        return "reference"
    if rel.startswith("docs/runbooks/"):        return "runbook"
    return "other"

REQUIRED = {          # 分类 -> 必填文首字段
    "adr":       ["日期", "状态"],
    "plan":      ["日期", "状态", "结论"],
    "living":    ["日期", "状态"],
    "reference": ["日期", "状态"],
    # runbook 的「状态」回答的是「哪几步真跑通过」——
    # 没验证过的步骤必须标出来，别让读者以为整篇都跑通了。
    "runbook":   ["日期", "状态"],
    "index":     ["日期"],
    "root":      [],
    "other":     [],
}
# 状态取值必须包含其中之一 —— 状态是「唯一判据」，拼错等于判据失效
STATUS_WORDS = ["采纳", "待实施", "设计", "已完成", "已废弃", "已取代"]
ADR_SECTIONS = ["Context", "Options", "Decision", "Consequences"]

def slugify(h):
    return re.sub(r"\s+", "-", re.sub(r"[^\w一-鿿 \-]", "", h.strip().lower())).strip("-")

files = sorted(
    [os.path.relpath(p, REPO) for p in glob.glob(REPO + "/*.md")] +
    [os.path.relpath(p, REPO) for p in glob.glob(REPO + "/docs/**/*.md", recursive=True)] +
    [os.path.relpath(p, REPO) for p in glob.glob(REPO + "/.githooks/*.md")]
)
if not files:
    print("docs.py: 没有找到 Markdown 文件", file=sys.stderr); sys.exit(0)

texts = {f: open(os.path.join(REPO, f), encoding="utf-8").read() for f in files}
heads = {f: {slugify(m.group(1)) for m in re.finditer(r"^#{1,6}\s+(.*)$", t, re.M)}
         for f, t in texts.items()}

# ── R2 命名 ───────────────────────────────────────────────────────────
for f in files:
    base, kind = os.path.basename(f), classify(f)
    if kind == "adr" and not re.fullmatch(r"[a-z0-9]+(-[a-z0-9]+)*\.md", base):
        err(f, "ADR 文件名必须是描述性 kebab-case（不带日期前缀）",
            "R2: decisions/ 用 <topic>.md，日期写在文首")
    if kind == "plan" and not re.fullmatch(r"\d{4}-\d{2}-\d{2}-[a-z0-9]+(-[a-z0-9]+)*\.md", base):
        err(f, "方案文件名必须是 YYYY-MM-DD-<topic>.md", "R2: plans/ 靠文件名排序")

# ── R3 文首必填 ───────────────────────────────────────────────────────
for f in files:
    t, kind = texts[f], classify(f)
    lines = t.split("\n")
    if not lines or not lines[0].startswith("# "):
        err(f + ":1", "H1 必须是文件第一行", "R3")
    head = "\n".join(lines[:10])

    for bad in ("Last updated", "Status"):
        m = re.search(r"^>\s*\**" + bad + r"\s*[:：]", head, re.M)
        if m:
            err(f + ":" + str(head[:m.start()].count("\n") + 1),
                f"文首字段用了英文 `{bad}:`",
                "R3: 字段名一律用中文（日期 / 状态）")

    for field in REQUIRED[kind]:
        if not re.search(r"^>\s*\**" + field + r"\**\s*[:：]", head, re.M):
            err(f, f"缺少文首必填字段 `{field}`（分类：{kind}）", "R3")

    m = re.search(r"^>\s*\**状态\**\s*[:：]\s*(.+)$", head, re.M)
    if m and not any(w in m.group(1) for w in STATUS_WORDS):
        err(f, f"状态取值 `{m.group(1).strip()}` 不在允许集合内",
            "允许包含：" + " / ".join(STATUS_WORDS) + "（决策被推翻时改为『已废弃』）")

    if kind == "adr":
        missing = [s for s in ADR_SECTIONS if not re.search(r"^##\s+" + s + r"\s*$", t, re.M)]
        if missing:
            err(f, "ADR 缺少必含章节：" + " / ".join(missing), "见 docs/decisions/README.md")

    if kind == "plan":
        m = re.search(r"^>\s*\**日期\**\s*[:：]\s*(\d{4}-\d{2}-\d{2})", head, re.M)
        fname_date = os.path.basename(f)[:10]
        if m and m.group(1) != fname_date:
            err(f, f"文件名日期 {fname_date} 与文首 `日期: {m.group(1)}` 不一致")

# ── 链接与锚点 ────────────────────────────────────────────────────────
for f in files:
    for i, line in enumerate(texts[f].split("\n"), 1):
        for m in re.finditer(r"\[[^\]]*\]\(([^)\s]+)\)", line):
            t = m.group(1)
            if t.startswith(("http://", "https://", "mailto:")):
                continue
            path, _, anc = t.partition("#")
            full = os.path.normpath(os.path.join(os.path.dirname(f), path)) if path else f
            if os.path.isdir(os.path.join(REPO, full)):
                full = os.path.join(full, "README.md")
            if not os.path.isfile(os.path.join(REPO, full)):
                err(f + ":" + str(i), f"断链：{t}")
            elif anc and full in heads and anc.lower() not in heads[full]:
                err(f + ":" + str(i), f"锚点不存在：{t}",
                    "该文件的标题有：" + ", ".join(sorted(heads[full])[:6]) + " …")

# ── 索引同步：新增记录必须进索引表 ────────────────────────────────────
for d in ("decisions", "plans"):
    idx = f"docs/{d}/README.md"
    if idx not in texts:
        continue
    listed = set()
    for m in re.finditer(r"\]\(([^)\s]+)\)", texts[idx]):
        tgt = m.group(1)
        if tgt.startswith(("http://", "https://", "mailto:")):
            continue
        tgt = tgt.split("#")[0]
        if not tgt or "/" in tgt:          # 只认同目录内的兄弟文件
            continue
        listed.add(tgt if tgt.endswith(".md") else tgt + ".md")
    actual = {os.path.basename(f) for f in files
              if f.startswith(f"docs/{d}/") and not f.endswith("README.md")}
    for missing in sorted(actual - listed):
        err(idx, f"索引表漏了 `{missing}`", "新增记录必须同时进索引表，否则读者看不到它")
    for phantom in sorted(listed - actual - {"README.md"}):
        if "/" not in phantom:
            err(idx, f"索引表列了不存在的 `{phantom}`")

# ── 计数声明一致性（真实踩过的坑：加了第 5 篇 ADR，索引仍写「4 条」）──
n_adr = len([f for f in files if classify(f) == "adr"])
for f in files:
    for i, line in enumerate(texts[f].split("\n"), 1):
        for m in re.finditer(r"(?:ADR[，,]\s*(\d+)\s*条|(\d+)\s*条\s*ADR)", line):
            claimed = int(m.group(1) or m.group(2))
            if claimed != n_adr:
                err(f + ":" + str(i), f"声称 ADR 有 {claimed} 条，实际 {n_adr} 条",
                    "改了 decisions/ 的篇数，记得回头改所有声明处")

# ── 输出 ──────────────────────────────────────────────────────────────
if ERR:
    print(f"\n  文档检查未通过（{len(ERR)} 处）：\n")
    for loc, msg, hint in ERR:
        print(f"    {loc}")
        print(f"      error: {msg}")
        if hint:
            print(f"      help:  {hint}")
        print()
    sys.exit(1)
print(f"  文档 ✓  {len(files)} 份，命名/文首/链接/索引/计数 全部一致")
