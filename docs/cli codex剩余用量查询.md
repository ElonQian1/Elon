可以获取，但**不要从 `auth.json` 里算**。`auth.json` 只是登录凭证，本身不包含“还剩多少额度”。真正的 Codex 用量/剩余额度在 OpenAI 服务端。

你这种“开发板上用 `auth.json` 跑 Codex CLI”的场景，优先用下面三种方式。

## 1. 最简单：在开发板的 Codex CLI 里直接看

在开发板上运行：

```bash
codex
```

进入 Codex TUI 后输入：

```text
/status
```

官方文档说 `/status` 可以在 Codex CLI 活跃会话里查看当前限制/剩余限制；CLI slash command 文档也说明 `/status` 会显示会话配置和 token 使用情况。([OpenAI开发者][1])

更推荐你再试这个：

```text
/usage
```

或者：

```text
/usage daily
/usage weekly
/usage cumulative
```

官方 CLI 文档说明 `/usage` 可以查看账户 token activity，也可以使用可用的 rate-limit reset；如果当前会话不是 Codex service account auth，它会提示需要登录。你是 `auth.json` 的 ChatGPT 登录方式，正常应该能用。([OpenAI开发者][2])

注意：这个不是 Linux shell 命令，是进入 `codex` 交互界面以后输入的命令。

---

## 2. 网页/桌面 App 看：Codex Settings → Usage

你也可以在 ChatGPT/Codex 的网页或 Codex App 里看：

```text
Codex Settings → Usage Dashboard
```

OpenAI 帮助文档说明，Plus/Pro 用户可以从 Codex Settings → Usage → Credits 购买 credits，也可以从 Codex Settings → Usage Dashboard 查看 credits balance 和 recent usage。([OpenAI Help Center][3])

这个方式最适合确认“我的 $200 Pro 到底还有多少额度、是否已经进 credits、是否快到限制”。

---

## 3. 脚本化获取：用 `codex app-server` 的 JSON-RPC

如果你想在开发板上写脚本定时检查，可以用 Codex 官方的 `app-server` 接口。官方文档里有这几个方法：

```text
account/read
account/rateLimits/read
account/usage/read
```

其中 `account/rateLimits/read` 会返回 ChatGPT rate limits，例如 `usedPercent`、`windowDurationMins`、`resetsAt`、`credits` 等字段；`account/usage/read` 会返回 token activity summary 和 daily buckets。([OpenAI开发者][4])

可以先用这个快速命令测试：

```bash
printf '%s\n' \
'{"method":"initialize","id":0,"params":{"clientInfo":{"name":"codex_usage_check","title":"Codex Usage Check","version":"0.1.0"}}}' \
'{"method":"initialized","params":{}}' \
'{"method":"account/read","id":1,"params":{"refreshToken":true}}' \
'{"method":"account/rateLimits/read","id":2}' \
'{"method":"account/usage/read","id":3}' \
| timeout 15s codex app-server \
| jq -c 'select(.id==1 or .id==2 or .id==3)'
```

如果你的开发板没有 `jq`，可以用下面这个 Python 脚本。

```python
# codex_usage_check.py
#!/usr/bin/env python3
import json
import subprocess
import sys
import time
from datetime import datetime
from typing import Any, Dict, Iterable, Set


def send_json(proc: subprocess.Popen, message: Dict[str, Any]) -> None:
    """向 codex app-server 发送一行 JSON-RPC 消息。"""
    if proc.stdin is None:
        raise RuntimeError("无法写入 codex app-server 的 stdin")

    proc.stdin.write(json.dumps(message, ensure_ascii=False) + "\n")
    proc.stdin.flush()


def read_until_ids(
    proc: subprocess.Popen,
    wanted_ids: Set[int],
    timeout_seconds: float = 20.0,
) -> Dict[int, Dict[str, Any]]:
    """一直读取 app-server 输出，直到拿到指定 id 的响应。"""
    if proc.stdout is None:
        raise RuntimeError("无法读取 codex app-server 的 stdout")

    results: Dict[int, Dict[str, Any]] = {}
    start = time.monotonic()

    while wanted_ids - set(results.keys()):
        if time.monotonic() - start > timeout_seconds:
            missing = sorted(wanted_ids - set(results.keys()))
            raise TimeoutError(f"等待 codex app-server 响应超时，缺少 id: {missing}")

        line = proc.stdout.readline()
        if line == "":
            raise RuntimeError("codex app-server 提前退出，可能是没有登录或命令不可用")

        line = line.strip()
        if not line:
            continue

        try:
            msg = json.loads(line)
        except json.JSONDecodeError:
            # 有些日志不是 JSON，忽略即可。
            continue

        msg_id = msg.get("id")
        if isinstance(msg_id, int) and msg_id in wanted_ids:
            results[msg_id] = msg

    return results


def format_time(unix_seconds: Any) -> str:
    """把 resetsAt 这类 Unix 秒时间戳转成人类可读时间。"""
    if not isinstance(unix_seconds, (int, float)):
        return "未知"

    return datetime.fromtimestamp(unix_seconds).astimezone().isoformat(timespec="seconds")


def print_rate_limit_bucket(name: str, bucket: Dict[str, Any]) -> None:
    """打印一个 rate limit bucket。"""
    primary = bucket.get("primary") or {}
    used_percent = primary.get("usedPercent")
    window_minutes = primary.get("windowDurationMins")
    resets_at = primary.get("resetsAt")

    print(f"\n[{name}]")
    print(f"limitId: {bucket.get('limitId')}")
    print(f"limitName: {bucket.get('limitName')}")
    print(f"planType: {bucket.get('planType')}")

    if isinstance(used_percent, (int, float)):
        remaining_percent = max(0.0, 100.0 - float(used_percent))
        print(f"已用比例: {used_percent:.2f}%")
        print(f"剩余比例: {remaining_percent:.2f}%")
    else:
        print("已用比例: 未返回")

    print(f"窗口长度: {window_minutes} 分钟")
    print(f"重置时间: {format_time(resets_at)}")

    if bucket.get("rateLimitReachedType"):
        print(f"触发限制类型: {bucket.get('rateLimitReachedType')}")


def main() -> int:
    """主函数：读取 Codex 账号、rate limit 和 token usage。"""
    try:
        proc = subprocess.Popen(
            ["codex", "app-server"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            bufsize=1,
        )
    except FileNotFoundError:
        print("错误：找不到 codex 命令，请先确认 Codex CLI 已安装并在 PATH 中。", file=sys.stderr)
        return 1

    try:
        # 1. app-server 要求每个连接先 initialize，再发 initialized。
        send_json(proc, {
            "method": "initialize",
            "id": 0,
            "params": {
                "clientInfo": {
                    "name": "codex_usage_check",
                    "title": "Codex Usage Check",
                    "version": "0.1.0",
                }
            },
        })
        send_json(proc, {"method": "initialized", "params": {}})

        # 2. 读取账号信息；refreshToken=true 会让 Codex 在 ChatGPT 托管登录模式下尝试刷新 token。
        send_json(proc, {
            "method": "account/read",
            "id": 1,
            "params": {"refreshToken": True},
        })

        # 3. 读取 rate limits，也就是你最关心的“用了多少、多久重置”。
        send_json(proc, {"method": "account/rateLimits/read", "id": 2})

        # 4. 读取 token activity，用于观察近期 token 使用情况。
        send_json(proc, {"method": "account/usage/read", "id": 3})

        responses = read_until_ids(proc, {1, 2, 3}, timeout_seconds=20.0)

        account_resp = responses.get(1, {})
        limits_resp = responses.get(2, {})
        usage_resp = responses.get(3, {})

        account = ((account_resp.get("result") or {}).get("account") or {})
        print("=== Codex 账号 ===")
        print(f"账号类型: {account.get('type')}")
        print(f"邮箱: {account.get('email')}")
        print(f"套餐: {account.get('planType')}")

        print("\n=== Rate Limits ===")
        limits_result = limits_resp.get("result") or {}
        by_id = limits_result.get("rateLimitsByLimitId")

        if isinstance(by_id, dict) and by_id:
            for limit_id, bucket in by_id.items():
                if isinstance(bucket, dict):
                    print_rate_limit_bucket(str(limit_id), bucket)
        else:
            single = limits_result.get("rateLimits")
            if isinstance(single, dict):
                print_rate_limit_bucket("default", single)
            else:
                print("没有返回 rate limit 信息。请确认当前是 ChatGPT/Codex 登录，不是纯 API key 登录。")

        reset_credits = limits_result.get("rateLimitResetCredits")
        if reset_credits is not None:
            print("\n=== 可用重置次数 ===")
            print(json.dumps(reset_credits, ensure_ascii=False, indent=2))

        credits = limits_result.get("credits")
        if credits is not None:
            print("\n=== Credits ===")
            print(json.dumps(credits, ensure_ascii=False, indent=2))

        print("\n=== Token Usage ===")
        usage_result = usage_resp.get("result") or {}
        summary = usage_result.get("summary")
        if summary is not None:
            print("summary:")
            print(json.dumps(summary, ensure_ascii=False, indent=2))

        daily_buckets = usage_result.get("dailyUsageBuckets")
        if isinstance(daily_buckets, list):
            print("\n最近 daily buckets:")
            for item in daily_buckets[-10:]:
                print(json.dumps(item, ensure_ascii=False))

        return 0

    except Exception as exc:
        print(f"错误：{exc}", file=sys.stderr)
        return 1

    finally:
        proc.terminate()
        try:
            proc.wait(timeout=3)
        except subprocess.TimeoutExpired:
            proc.kill()


if __name__ == "__main__":
    raise SystemExit(main())
```

运行：

```bash
python3 codex_usage_check.py
```

你主要看这几个字段：

```text
usedPercent         当前窗口已用百分比
100 - usedPercent   当前窗口大约剩余百分比
windowDurationMins  这个限制窗口多长
resetsAt            什么时候重置
credits             如果返回了，表示剩余 credits 信息
rateLimitResetCredits.availableCount  可用的重置次数
```

---

## 你的 $200 Pro 大概是什么额度？

你说的 $200/月应该是 **ChatGPT Pro $200 档**。OpenAI 帮助文档说，Pro $100 是 Plus 的 5x 用量，Pro $200 是 Plus 的 20x 用量，而且 $200 Pro 仍然是最高用量档。([OpenAI Help Center][5])

Codex 官方价格页里，Pro 20x 的本地消息量是一个范围，不是固定条数。例如每 5 小时大概是：

| 模型           | Pro 20x 本地消息 / 5h |
| ------------ | ----------------: |
| GPT-5.5      |          300–1600 |
| GPT-5.4      |          400–2000 |
| GPT-5.4 mini |         1200–7000 |

官方同时说明，这些 local messages 和 cloud tasks 共用 5 小时窗口，而且可能还有额外 weekly limits；实际能发多少消息取决于模型、代码任务大小、上下文长度、运行位置等。([OpenAI开发者][1])

所以不要把它理解为“固定剩余 N 条”。更准确的判断方式是：

```text
剩余程度 ≈ 100% - usedPercent
```

如果返回多个 bucket，就看所有 bucket，哪个最接近 100% 已用，哪个就是更可能先卡住你的限制。

---

## 额外提醒

`codex login status` 只能确认当前是否登录、用的是哪种认证方式，它不等于用量查询。官方 CLI 参考说明它只是打印 active authentication mode，并在有凭证时返回 0。([OpenAI开发者][6])

另外，`auth.json` 要继续当密码保护。OpenAI 文档明确说 `~/.codex/auth.json` 包含 access tokens，不要提交、不要贴出来、不要分享到聊天或工单里。([OpenAI开发者][7])

[1]: https://developers.openai.com/codex/pricing "Pricing – Codex | OpenAI Developers"
[2]: https://developers.openai.com/codex/cli/slash-commands "Slash commands in Codex CLI | OpenAI Developers"
[3]: https://help.openai.com/en/articles/12642688-using-credits-for-flexible-usage-in-chatgpt-freegopluspro "Using Credits for Flexible Usage in ChatGPT (Free/Go/Plus/Pro)  | OpenAI Help Center"
[4]: https://developers.openai.com/codex/app-server "App Server – Codex | OpenAI Developers"
[5]: https://help.openai.com/en/articles/9793128-about-chatgpt-pro-tiers "About ChatGPT Pro tiers | OpenAI Help Center"
[6]: https://developers.openai.com/codex/cli/reference "Command line options – Codex CLI | OpenAI Developers"
[7]: https://developers.openai.com/codex/auth "Authentication – Codex | OpenAI Developers"
