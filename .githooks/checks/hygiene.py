#!/usr/bin/env python3
"""卫生检查 —— 密钥、危险文件名、体积、以及 .editorconfig 基线（LF / 末尾换行 / 无行尾空格 / 空格缩进）。

用法：hygiene.py --staged   仅检查暂存内容（读的是暂存的 blob，不是工作区）
      hygiene.py --all      检查仓库内全部被跟踪 + 未跟踪（不含被忽略）文件
"""
import os, re, subprocess, sys

REPO = subprocess.run(["git", "rev-parse", "--show-toplevel"],
                      capture_output=True, text=True).stdout.strip()
EMPTY_TREE = "4b825dc642cb6eb9a060e54bf8d69288fbee4904"
MAX_BYTES = 2 * 1024 * 1024

# 高信号密钥模式。宁可窄一些也不要天天误报 —— 会误报的门禁最后都会被 --no-verify 绕过。
SECRETS = [
    (r"tskey-(?:auth|client)-[A-Za-z0-9]{10,}",        "Tailscale 密钥（本项目 CI 会用到，务必只放 GitHub Secrets）"),
    (r"gh[pousr]_[A-Za-z0-9]{ticks}".replace("{ticks}", "{36,}"), "GitHub token"),
    (r"github_pat_[A-Za-z0-9_]{22,}",                  "GitHub fine-grained PAT"),
    (r"AKIA[0-9A-Z]{16}",                              "AWS Access Key ID"),
    (r"-----BEGIN (?:RSA |EC |OPENSSH |PGP )?PRIVATE KEY-----", "私钥"),
    (r"\bxox[baprs]-[A-Za-z0-9-]{10,}",                "Slack token"),
    (r"(?i)\bCLOUDFLARE_API_TOKEN\s*[=:]\s*['\"]?[A-Za-z0-9_\-]{30,}", "Cloudflare API token"),
    (r"(?i)\b(?:api[_-]?key|secret|password|passwd)\s*[=:]\s*['\"][^'\"\s${}]{16,}['\"]", "疑似硬编码凭据"),
]
DANGEROUS = [
    (r"^\.dev\.vars(\..+)?$",        "wrangler 本地变量文件"),
    (r"^\.env(\..+)?$",              "环境变量文件"),
    (r".*\.(pem|p12|pfx|keystore)$", "证书 / 密钥库"),
    (r"^id_(rsa|dsa|ecdsa|ed25519)$","SSH 私钥"),
]
ALLOW_NAME = [r"^\.env\.example$", r".*\.pem\.example$"]
# .editorconfig 里 indent_style=space 对 [*] 生效；Makefile 天生要 tab，单独放行
TAB_OK = [r"(^|/)Makefile$", r".*\.mk$", r".*\.go$"]

ERR = []
def err(path, msg, hint=None): ERR.append((path, msg, hint))

def git(*args):
    return subprocess.run(["git", *args], capture_output=True, cwd=REPO).stdout

def staged_files():
    base = EMPTY_TREE if subprocess.run(["git", "rev-parse", "--verify", "-q", "HEAD"],
                                        capture_output=True, cwd=REPO).returncode else "HEAD"
    out = git("diff", "--cached", "--name-only", "--diff-filter=ACM", base)
    return [p for p in out.decode().split("\n") if p]

def all_files():
    out = git("ls-files", "--cached", "--others", "--exclude-standard")
    return [p for p in out.decode().split("\n") if p]

def read(path, staged):
    if staged:
        r = subprocess.run(["git", "show", f":{path}"], capture_output=True, cwd=REPO)
        return r.stdout if r.returncode == 0 else None
    full = os.path.join(REPO, path)
    return open(full, "rb").read() if os.path.isfile(full) else None

def main():
    staged = "--staged" in sys.argv
    files = staged_files() if staged else all_files()
    checked = 0

    for path in files:
        base = os.path.basename(path)
        for pat, what in DANGEROUS:
            if re.match(pat, base) and not any(re.match(a, base) for a in ALLOW_NAME):
                err(path, f"不该进仓库的文件：{what}",
                    "已在 .gitignore 里；若确实是模板，改名为 *.example")

        raw = read(path, staged)
        if raw is None:
            continue
        checked += 1

        if len(raw) > MAX_BYTES:
            err(path, f"文件 {len(raw)/1024/1024:.1f} MiB，超过 2 MiB 上限",
                "Worker 包体 Free 档上限 3 MiB gzip，大资产请走外部托管或压缩后再提")

        try:
            text = raw.decode("utf-8")
        except UnicodeDecodeError:
            continue  # 二进制只做体积检查

        for pat, what in SECRETS:
            m = re.search(pat, text)
            if m:
                ln = text[:m.start()].count("\n") + 1
                err(f"{path}:{ln}", f"疑似泄露 {what}",
                    "撤下后请**视同已泄露**并轮换该凭据 —— 进过 git 对象库就删不干净了")

        if "\r\n" in text:
            err(path, "含 CRLF 换行", ".editorconfig: end_of_line = lf")
        if raw and not raw.endswith(b"\n"):
            err(path, "缺少末尾换行", ".editorconfig: insert_final_newline = true")
        lines = text.split("\n")
        bad_ws = [i for i, l in enumerate(lines, 1) if l != l.rstrip()]
        if bad_ws:
            err(f"{path}:{bad_ws[0]}", f"行尾有空格（共 {len(bad_ws)} 行）",
                ".editorconfig: trim_trailing_whitespace = true")
        if not any(re.search(p, path) for p in TAB_OK):
            tabs = [i for i, l in enumerate(lines, 1) if l.startswith("\t")]
            if tabs:
                err(f"{path}:{tabs[0]}", f"用了 tab 缩进（共 {len(tabs)} 行）",
                    ".editorconfig: indent_style = space")

    if ERR:
        print(f"\n  卫生检查未通过（{len(ERR)} 处）：\n")
        for path, msg, hint in ERR:
            print(f"    {path}")
            print(f"      error: {msg}")
            if hint:
                print(f"      help:  {hint}")
            print()
        return 1
    scope = "暂存" if staged else "全仓"
    print(f"  卫生 ✓  {scope} {checked} 个文件，密钥/体积/换行/空白 全部干净")
    return 0

sys.exit(main())
