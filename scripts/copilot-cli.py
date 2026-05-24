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

    req = urllib.request.Request(
        f"{api_base}/chat/completions",
        data=payload,
        headers=headers,
        method="POST",
    )

    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            data = json.load(resp)
        content = data["choices"][0]["message"]["content"]
        print(content)
    except urllib.error.HTTPError as e:
        body = e.read().decode("utf-8", errors="replace")
        print(f"API 错误 {e.code}: {body}", file=sys.stderr)
        sys.exit(1)
    except urllib.error.URLError as e:
        print(f"网络错误: {e.reason}", file=sys.stderr)
        sys.exit(1)
    except (KeyError, IndexError, json.JSONDecodeError) as e:
        print(f"响应解析失败: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
