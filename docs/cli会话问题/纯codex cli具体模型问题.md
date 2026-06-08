截至目前官方文档，**Codex CLI 推荐使用的具体模型**主要是这几个：

| 模型                    | 官方定位                                                  | 适合场景                         | 启动示例                           |
| --------------------- | ----------------------------------------------------- | ---------------------------- | ------------------------------ |
| `gpt-5.5`             | Codex 推荐首选模型，面向复杂编码、电脑使用、知识工作、研究工作流                   | 大型 Rust 项目、复杂 bug、架构设计、跨文件重构 | `codex -m gpt-5.5`             |
| `gpt-5.4`             | 旗舰 frontier model，适合专业工作、强编码、推理、工具使用、agentic workflow | 日常高质量编码、review、调试            | `codex -m gpt-5.4`             |
| `gpt-5.4-mini`        | 更快、更省的 mini model                                     | 小改动、快速问答、轻量子任务、subagent      | `codex -m gpt-5.4-mini`        |
| `gpt-5.3-codex-spark` | 文本-only research preview，Pro 用户可用，优化近实时编码迭代           | 很快的代码迭代；不适合图片理解              | `codex -m gpt-5.3-codex-spark` |

官方建议大多数 Codex 任务从 `gpt-5.5` 开始；想要更快、成本更低时用 `gpt-5.4-mini`；`gpt-5.3-codex-spark` 是 Pro 用户的 research preview，并且官方标注为 **text-only**。([OpenAI开发者][1])

另外，`gpt-5.2` 和 `gpt-5.3-codex` 在 ChatGPT 登录方式下已经被 Codex 标记为 deprecated；如果脚本或配置还写着这些模型，应该迁移到上面的新模型。API-key 模式下某些旧模型是否还能用，要以 API models 页面和你的账号权限为准。([OpenAI开发者][1])

---

## 1. 查看你本机 Codex CLI 实际能看到哪些模型

最准确的方法不是死记模型名，而是直接看当前 Codex CLI 读到的 model catalog：

```bash
# 终端命令：查看 Codex 当前看到的模型目录
codex debug models
```

如果你只想看当前安装包内置的模型目录，不刷新远程目录：

```bash
# 终端命令：只查看当前 Codex binary 自带的模型目录
codex debug models --bundled
```

官方命令参考里写明，`codex debug models` 会以 JSON 打印 Codex 看到的原始 model catalog，`--bundled` 则跳过刷新，只看当前 binary 内置目录。([OpenAI开发者][2])

---

## 2. 临时切换模型：`-m` / `--model`

最直接：

```bash
# 终端命令：用 gpt-5.5 启动一个新的 Codex CLI 会话
codex -m gpt-5.5
```

或者：

```bash
# 终端命令：用 gpt-5.4-mini 执行一次轻量任务
codex -m gpt-5.4-mini "请快速解释这个项目的目录结构"
```

`codex exec` 也可以指定模型：

```bash
# 终端命令：非交互模式下指定模型
codex exec -m gpt-5.4 "检查当前 git diff 是否有明显 bug"
```

官方命令参考写明，`--model, -m` 用来覆盖配置中的模型，例如 `gpt-5.4`。([OpenAI开发者][2])

---

## 3. 在已经打开的 Codex CLI 会话中切换模型：`/model`

进入交互式 Codex 后，直接输入：

```text
/model
```

然后从弹出的列表中选择模型。选完之后可以用：

```text
/status
```

确认当前模型、token、权限、上下文等状态。

官方 slash command 文档说明，`/model` 用来选择 active model，并且在可用时也可以选择 reasoning effort；`/status` 可以显示当前 session configuration 和 token usage。([OpenAI开发者][3])

---

## 4. 永久设置默认模型：`~/.codex/config.toml`

Codex CLI 和 IDE extension 共用同一套 `config.toml` 配置。用户级配置默认在：

```text
~/.codex/config.toml
```

你可以这样设置默认模型：

```toml
# ~/.codex/config.toml

# Codex 默认使用的模型
model = "gpt-5.5"
```

官方配置文档说明，用户级配置文件是 `~/.codex/config.toml`，CLI 和 IDE extension 共用配置层，并且可以用它设置默认 model 和 provider。([OpenAI开发者][4])

---

## 5. 同时设置模型和推理强度

比较常用的是这样：

```toml
# ~/.codex/config.toml

# 默认模型：复杂任务优先用 gpt-5.5
model = "gpt-5.5"

# 推理强度：minimal | low | medium | high | xhigh
# high 适合复杂代码、跨文件重构、难 bug
model_reasoning_effort = "high"

# 推理摘要：auto | concise | detailed | none
# auto 一般够用；detailed 适合你想看更详细的过程摘要
model_reasoning_summary = "auto"

# 输出详细程度：low | medium | high
# medium 比较平衡；high 更啰嗦但解释更完整
model_verbosity = "medium"
```

官方配置参考里，`model_reasoning_effort` 支持 `minimal | low | medium | high | xhigh`，但只对支持的 Responses API 模型有效，且 `xhigh` 依赖具体模型；`model_reasoning_summary` 支持 `auto | concise | detailed | none`；`model_verbosity` 支持 `low | medium | high`，是 GPT-5 Responses API 的 verbosity override。([OpenAI开发者][5])

---

## 6. 一次性覆盖模型参数：`-c` / `--config`

你不想改配置文件时，可以临时覆盖：

```bash
# 终端命令：临时用 gpt-5.4
codex --model gpt-5.4
```

也可以用通用配置覆盖：

```bash
# 终端命令：临时覆盖 model；注意字符串需要按 TOML 方式加引号
codex --config model='"gpt-5.4"'
```

覆盖推理强度：

```bash
# 终端命令：临时设置高推理强度
codex --config model_reasoning_effort='"high"'
```

同时覆盖多个参数：

```bash
# 终端命令：临时指定模型、推理强度、输出详细程度
codex \
  --config model='"gpt-5.5"' \
  --config model_reasoning_effort='"xhigh"' \
  --config model_verbosity='"high"'
```

官方高级配置文档说明，优先用专门 flag，比如 `--model`；需要覆盖任意 key 时用 `-c` / `--config`，并且 `--config` 的值按 TOML 解析，所以字符串最好写成 `model='"gpt-5.4"'` 这种形式。([OpenAI开发者][6])

---

## 7. 用 profile 做“模型预设”

这个很适合你这种经常开发 Rust 项目的场景。比如你可以做三个 profile：

### 深度开发 / 难 bug

```toml
# ~/.codex/deep.config.toml

# 复杂任务模型
model = "gpt-5.5"

# 更强推理
model_reasoning_effort = "xhigh"

# 回答更详细
model_verbosity = "high"

# 计划模式下也使用高推理
plan_mode_reasoning_effort = "high"
```

启动：

```bash
# 终端命令：使用 deep profile
codex --profile deep
```

### 快速小改动

```toml
# ~/.codex/fast.config.toml

# 快速、低成本模型
model = "gpt-5.4-mini"

# 低到中等推理即可
model_reasoning_effort = "low"

# 输出短一点
model_verbosity = "low"
```

启动：

```bash
# 终端命令：使用 fast profile
codex --profile fast
```

### 代码 review

```toml
# ~/.codex/review.config.toml

# 主会话模型
model = "gpt-5.5"

# /review 专用模型；不设置则默认使用当前会话模型
review_model = "gpt-5.5"

# review 更适合较高推理
model_reasoning_effort = "high"

# 输出中等详细
model_verbosity = "medium"
```

启动：

```bash
# 终端命令：用 review profile 运行 Codex
codex --profile review
```

官方文档说明，profile 文件放在 `~/.codex/<profile-name>.config.toml`，通过 `codex --profile profile-name` 或 `codex exec --profile profile-name` 加载；profile 会覆盖基础用户配置中不同的值。([OpenAI开发者][6])

---

## 8. 模型相关配置项速查

| 配置项                              | 可选值 / 类型                                                 | 作用                                  |
| -------------------------------- | -------------------------------------------------------- | ----------------------------------- |
| `model`                          | 字符串，例如 `"gpt-5.5"`                                       | 当前默认模型                              |
| `model_provider`                 | 字符串，默认 `"openai"`                                        | 模型提供方 ID                            |
| `model_reasoning_effort`         | `minimal` / `low` / `medium` / `high` / `xhigh`          | 推理强度                                |
| `plan_mode_reasoning_effort`     | `none` / `minimal` / `low` / `medium` / `high` / `xhigh` | 计划模式专用推理强度                          |
| `model_reasoning_summary`        | `auto` / `concise` / `detailed` / `none`                 | 推理摘要详细程度                            |
| `model_verbosity`                | `low` / `medium` / `high`                                | GPT-5 系列 Responses API 输出详细程度       |
| `model_context_window`           | 数字                                                       | 手动指定 active model 的上下文窗口 token 数    |
| `model_auto_compact_token_limit` | 数字                                                       | 自动压缩历史上下文的 token 阈值                 |
| `model_catalog_json`             | 文件路径                                                     | 启动时加载自定义 model catalog              |
| `review_model`                   | 字符串                                                      | `/review` 使用的模型；默认用当前 session model |
| `service_tier`                   | 字符串，例如 `fast` / `flex`                                   | 新 turn 的服务层级，具体取决于模型目录支持            |
| `tool_output_token_limit`        | 数字                                                       | 每个工具输出写入历史的 token 预算                |
| `tools.view_image`               | 布尔值                                                      | 启用本地图片附件工具 `view_image`             |

其中 `model`、`model_provider`、`model_context_window`、`model_auto_compact_token_limit`、`model_catalog_json` 都在官方配置参考中列为模型相关配置；`model_reasoning_effort`、`model_reasoning_summary`、`model_verbosity` 等是推理和输出相关设置；`review_model`、`service_tier`、`tool_output_token_limit`、`tools.view_image` 也会影响模型调用或上下文历史行为。([OpenAI开发者][5])

---

## 9. 本地模型 / 第三方 provider

Codex CLI 不只限于 OpenAI 官方 provider。官方模型页说，除了推荐模型，也可以把 Codex 指向任何支持 Responses API 或 Chat Completions API 的模型和 provider；不过 Chat Completions 支持已标记为 deprecated，未来会从 Codex 移除。([OpenAI开发者][1])

自定义 provider 示例：

```toml
# ~/.codex/config.toml

# 要使用的模型名，取决于你的 provider 支持什么
model = "gpt-5.4"

# 指向自定义 provider
model_provider = "proxy"

[model_providers.proxy]
name = "OpenAI using LLM proxy"
base_url = "https://proxy.example.com/v1"
env_key = "OPENAI_API_KEY"
wire_api = "responses"
```

官方高级配置文档说明，custom provider 可以定义 `base_url`、`wire_api`、认证、headers 等；当前配置参考里 `wire_api` 的支持值是 `responses`。([OpenAI开发者][6])

本地 OSS 模式也支持，例如 Ollama / LM Studio：

```toml
# ~/.codex/config.toml

# --oss 默认使用的本地 provider
oss_provider = "ollama"
```

启动：

```bash
# 终端命令：使用本地 open source provider
codex --oss -m <你的本地模型ID>
```

官方文档说明，Codex 可以通过 `--oss` 使用本地 open source provider，例如 Ollama 或 LM Studio；`oss_provider` 可以设置为 `ollama` 或 `lmstudio`。([OpenAI开发者][6])

---

## 10. 我建议你的实际配置

你主要做 Rust 底层应用，我建议默认用强一点，但保留快速 profile：

```toml
# ~/.codex/config.toml

# 默认模型：复杂 Rust 项目、跨文件分析、调试优先
model = "gpt-5.5"

# 默认推理强度：复杂代码比较适合 high
model_reasoning_effort = "high"

# 输出详细程度：你需要中文解释和完整代码，建议 medium 或 high
model_verbosity = "high"

# 推理摘要：auto 比较稳
model_reasoning_summary = "auto"

# 计划模式更认真一点
plan_mode_reasoning_effort = "high"
```

再建一个快速配置：

```toml
# ~/.codex/fast.config.toml

# 快速小任务模型
model = "gpt-5.4-mini"

# 降低推理和输出长度
model_reasoning_effort = "low"
model_verbosity = "medium"
```

日常复杂任务：

```bash
# 终端命令：默认强模型
codex
```

小问题、小改动：

```bash
# 终端命令：快速 profile
codex --profile fast
```

临时超强模式：

```bash
# 终端命令：单次使用更高推理强度
codex \
  -m gpt-5.5 \
  -c model_reasoning_effort='"xhigh"' \
  -c model_verbosity='"high"'
```

一句话总结：**模型用 `-m/--model` 临时切，默认写进 `~/.codex/config.toml`，会话中用 `/model` 切；复杂任务用 `gpt-5.5 + high/xhigh`，轻量任务用 `gpt-5.4-mini + low/medium`，实际可用模型以 `codex debug models` 输出为准。**

[1]: https://developers.openai.com/codex/models "Models – Codex | OpenAI Developers"
[2]: https://developers.openai.com/codex/cli/reference "Command line options – Codex CLI | OpenAI Developers"
[3]: https://developers.openai.com/codex/cli/slash-commands "Slash commands in Codex CLI | OpenAI Developers"
[4]: https://developers.openai.com/codex/config-basic "Config basics – Codex | OpenAI Developers"
[5]: https://developers.openai.com/codex/config-reference "Configuration Reference – Codex | OpenAI Developers"
[6]: https://developers.openai.com/codex/config-advanced "Advanced Configuration – Codex | OpenAI Developers"
