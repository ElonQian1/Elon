#!/usr/bin/env python3
"""
copilot-cli — Copilot / GitHub Models API 命令行包装器

用法：
  copilot-cli [--model MODEL] PROMPT        # 参数模式（Arg）
  echo "PROMPT" | copilot-cli [--model MODEL]  # stdin 模式

环境变量：
  GITHUB_TOKEN 或 COPILOT_GITHUB_TOKEN     — GitHub PAT（必须）
  COPILOT_API_BASE                          — API 基础地址（可选，默认 GitHub Models）
  COPILOT_SYSTEM_PROMPT                     — 系统提示词（可选）
  COPILOT_PROXY                             — 专用代理（可选，如 socks5h://...）；不继承 ALL_PROXY

服务端配置示例（.env 或 systemd service）：
  COPILOT_CLI_ENABLED=true
  COPILOT_CLI_BIN=/usr/local/bin/copilot-cli
  COPILOT_CLI_MODELS=gpt-4o,gpt-4o-mini,claude-3.5-sonnet,o1-mini
  COPILOT_CLI_PROMPT_MODE=arg
  GITHUB_TOKEN=<你的 GitHub PAT>
"""

import argparse
import json
import os
import sys
import urllib.error
import urllib.request


def main():
    parser = argparse.ArgumentParser(
        description="Copilot / GitHub Models API CLI wrapper"
    )
    parser.add_argument(
        "--model", "-m",
        default=os.environ.get("COPILOT_DEFAULT_MODEL", "gpt-4o"),
        help="模型名称（默认 gpt-4o）",
    )
    parser.add_argument(
        "prompt",
        nargs="*",
        help="用户提示词（为空时从 stdin 读取）",
    )
    args = parser.parse_args()

    # 读取 prompt：命令行参数优先，否则读 stdin
    if args.prompt:
        prompt = " ".join(args.prompt)
    else:
        prompt = sys.stdin.read()

    if not prompt.strip():
        print("错误：prompt 为空", file=sys.stderr)
        sys.exit(1)

    # 读取鉴权 token
    token = (
        os.environ.get("GITHUB_TOKEN")
        or os.environ.get("COPILOT_GITHUB_TOKEN", "")
    )
    if not token:
        print(
            "错误：未设置 GITHUB_TOKEN 或 COPILOT_GITHUB_TOKEN 环境变量",
            file=sys.stderr,
        )
        sys.exit(1)

    api_base = os.environ.get(
        "COPILOT_API_BASE", "https://models.inference.ai.azure.com"
    ).rstrip("/")

    # 构造 messages
    messages = []
    system_prompt = os.environ.get("COPILOT_SYSTEM_PROMPT", "")
    if system_prompt:
        messages.append({"role": "system", "content": system_prompt})
    messages.append({"role": "user", "content": prompt})

    # 构造请求头
    headers = {
        "Authorization": f"Bearer {token}",
        "Content-Type": "application/json",
    }
    # Copilot 直连端点需要额外头
    if "githubcopilot.com" in api_base:
        headers["editor-version"] = "vscode/1.99.0"
        headers["editor-plugin-version"] = "copilot-chat/0.26.0"
        headers["Copilot-Integration-Id"] = os.environ.get(
            "COPILOT_INTEGRATION_ID", "vscode-chat"
        )

    payload = json.dumps(
        {
            "model": args.model,
            "messages": messages,
            "stream": False,
        }
    ).encode("utf-8")

    # 代理：只读取 COPILOT_PROXY 专用变量，不继承 ALL_PROXY（服务器直连 Azure 更快）
    copilot_proxy = os.environ.get("COPILOT_PROXY", "")

    import subprocess
    curl_cmd = ["curl", "-s", "--noproxy", "*", "--max-time", "120", "-X", "POST",
                f"{api_base}/chat/completions"]
    for k, v in headers.items():
        curl_cmd += ["-H", f"{k}: {v}"]
    curl_cmd += ["-d", payload.decode("utf-8")]

    if copilot_proxy:
        # 有专用代理时移除 --noproxy 并添加代理
        curl_cmd.remove("--noproxy")
        curl_cmd.remove("*")
        curl_cmd += ["--proxy", copilot_proxy]

    try:
        result = subprocess.run(curl_cmd, capture_output=True, timeout=130)
        if result.returncode != 0:
            print(f"网络错误: {result.stderr.decode('utf-8', errors='replace')}", file=sys.stderr)
            sys.exit(1)
        data = json.loads(result.stdout.decode("utf-8"))
        if "error" in data:
            err = data["error"]
            print(f"API 错误: {err.get('message', err)}", file=sys.stderr)
            sys.exit(1)
        content = data["choices"][0]["message"]["content"]
        print(content)
    except subprocess.TimeoutExpired:
        print("请求超时", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
