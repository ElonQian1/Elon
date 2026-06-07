是的，**现在这两个 CLI 都支持“会话”和“会话上下文继续”**，但实现方式不完全一样。这里的 “Copilot CLI” 我按 **新的 GitHub Copilot CLI：`copilot`** 来说；老的 **GitHub CLI Copilot extension / `gh copilot`** 已经被 GitHub 标注为 retired，并被新的 GitHub Copilot CLI 替代了。([GitHub Docs][1])

| 工具                     | 是否支持会话 | 是否支持恢复上下文 | 主要命令 / 功能                                                                                             |
| ---------------------- | -----: | --------: | ----------------------------------------------------------------------------------------------------- |
| **OpenAI Codex CLI**   |     支持 |        支持 | `codex resume`、`codex resume --last`、`codex resume --all`、`codex resume <SESSION_ID>`、TUI 里 `/resume` |
| **GitHub Copilot CLI** |     支持 |        支持 | `copilot --continue`、`--resume`、TUI 里 `/resume`、`/continue`、`/session`、`/context`、`/compact`          |

**Codex CLI** 会把 transcript 本地保存起来，所以你可以从之前的会话继续，不用重新解释上下文；官方文档写明 `codex resume` 会打开最近交互式会话选择器，`--last` 可以直接恢复当前目录最近的会话，`--all` 可以看所有目录的会话，`<SESSION_ID>` 可以恢复指定会话。恢复后的 run 会保留原来的 transcript、plan history 和 approvals，让 Codex 能用之前的上下文继续工作。([OpenAI 开发者][2])
在 Codex 的 TUI 内部，也可以用 `/resume` 从保存的 session picker 里恢复对话；`/compact` 可以把较早对话压缩成摘要，释放上下文空间，但不是逐字保留全部历史。([OpenAI 开发者][3])

**GitHub Copilot CLI** 也支持恢复交互式 session。官方文档说可以用 `--resume` 或 `/resume` 选择并恢复 CLI session，并“with the saved context” 从之前的位置继续；快速恢复最近关闭的本地 session 可以用 `copilot --continue`。([GitHub Docs][4])
它的命令表里也列出 `/resume [SESSION-ID]`、`/continue [SESSION-ID]` 用于切换/恢复会话，`/session ...` 用于查看和管理 session，`/restart` 可以重启 CLI 但保留当前 session。([GitHub Docs][5])

上下文管理方面，**Copilot CLI 的文档写得更明确**：它的 context window 会包含你的消息、Copilot 回复、工具调用及结果、系统指令等；可以用 `/context` 查看 token 使用情况，也可以用 `/compact` 手动压缩上下文。长会话接近上下文限制时，它会自动 compaction，把历史压缩成摘要继续用。([GitHub Docs][6])
不过要注意：这种 compaction 是“摘要化”，不是永久精确记忆；GitHub 文档明确说细粒度内容，比如每句话的精确措辞、完整命令输出、早期小决策，可能会丢失。([GitHub Docs][6])

所以结论可以这样理解：

**Codex CLI：更像“本地保存线程 + 可恢复 + 可 fork/compact 的 coding agent 会话”。**
适合你在一个 Rust 项目里长期让它读代码、改代码、跑测试，然后下次继续同一个任务。

**Copilot CLI：更像“带 session 管理、上下文窗口可视化、自动压缩、历史检索的 GitHub coding agent CLI”。**
它的 `/context`、`/compact`、`/session checkpoints`、session history 这些上下文管理能力比较系统化；它还可以根据 session history 回答你过去 CLI 使用情况的问题。([GitHub Docs][6])

我的建议是：如果你主要在本地仓库里让 AI 连续做较大的代码修改，**两个都可以用会话恢复**；如果你特别在意“能看到上下文剩余多少、压缩了什么、会话管理命令完整度”，目前 **Copilot CLI 的上下文管理界面更显式**。如果你主要使用 OpenAI 生态、想和 Codex app / IDE / CLI 之间切换，**Codex CLI 的 session 体系会更顺手**。

[1]: https://docs.github.com/copilot/github-copilot-in-the-cli "Using the GitHub CLI Copilot extension - GitHub Docs"
[2]: https://developers.openai.com/codex/cli/features "Features – Codex CLI | OpenAI Developers"
[3]: https://developers.openai.com/codex/cli/slash-commands "Slash commands in Codex CLI | OpenAI Developers"
[4]: https://docs.github.com/en/copilot/how-tos/copilot-cli/use-copilot-cli/overview "Using GitHub Copilot CLI - GitHub Docs"
[5]: https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference "GitHub Copilot CLI command reference - GitHub Docs"
[6]: https://docs.github.com/en/copilot/concepts/agents/copilot-cli/context-management "Managing context in GitHub Copilot CLI - GitHub Docs"
