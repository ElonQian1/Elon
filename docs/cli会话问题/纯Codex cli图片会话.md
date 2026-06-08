支持。**Codex CLI 现在明确支持图片阅读理解**，官方文档叫 **Image inputs**：可以把截图、设计稿等图片和你的 prompt 一起发给 Codex，让它读取图片细节。官方例子包括 `codex -i screenshot.png "Explain this error"` 和 `codex --image img1.png,img2.jpg "Summarize these diagrams"`。([OpenAI开发者][1])

## 1. Codex CLI 能不能“看图理解”？

能。它可以理解常见图片里的内容，比如：

```text
UI 截图
报错截图
设计稿
流程图
架构图
网页截图
手机 App 截图
控制台错误截图
```

官方文档写的是：你可以把 screenshots 或 design specs 附加进去，让 Codex “read image details alongside your prompt”。也就是说，它不是只拿到图片路径文本，而是把图片作为视觉输入交给模型理解。([OpenAI开发者][1])

---

## 2. 最推荐的图片输入方式：命令行 `-i / --image`

例如你有一张截图：

```bash
screenshot.png
```

可以这样启动 Codex：

```bash
codex -i screenshot.png "请读取这张截图，告诉我里面的错误信息、界面状态，以及可能原因"
```

或者完整参数名：

```bash
codex --image screenshot.png "请分析这张图"
```

`--image` 和 `-i` 的作用是：把一个或多个图片文件附加到初始 prompt；多个图片可以用逗号分隔，也可以重复使用这个参数。([OpenAI开发者][2])

比如两张图：

```bash
codex --image before.png,after.png "请比较这两张截图，找出 UI 状态变化"
```

或者：

```bash
codex -i before.png -i after.png "请比较这两张图"
```

官方文档说 Codex 支持常见格式，例如 **PNG** 和 **JPEG**。([OpenAI开发者][1])

---

## 3. 交互式会话里也能发图片

启动：

```bash
codex
```

进入 TUI 之后，官方文档说可以直接把图片粘贴到 interactive composer 里，也就是 Codex CLI 的输入框。([OpenAI开发者][1])

你可以这样问：

```text
请分析我刚才粘贴的这张截图：
1. 识别里面所有可见文字
2. 判断当前页面是什么状态
3. 找出异常提示
4. 给出下一步调试建议
```

但是注意：**粘贴图片这件事是否成功，和你的终端环境强相关**。如果终端没有把剪贴板图片传给 Codex CLI，Codex 就收不到图。为了稳定，我更推荐你用 `-i` 或 `--image`，不要优先依赖粘贴。

---

## 4. 在同一个 Session ID 里继续看新图片

这点很重要。你之前问过会话上下文，图片也可以和 resume 结合。

如果你已经有一个旧会话，可以恢复：

```bash
codex resume <SESSION_ID>
```

恢复之后，在交互式界面里继续粘贴新图片，然后说：

```text
这是新的页面截图。请基于前面会话里的项目上下文，分析这张图和之前状态有什么不同。
```

Codex 官方文档说，resume 会恢复原来的 transcript、plan history 和 approvals，所以 Codex 可以用之前上下文继续理解你的新指令。([OpenAI开发者][1])

如果你想用非交互模式继续旧会话并附加图片，可以用：

```bash
codex exec resume --last --image new_screen.png "基于上次会话上下文，分析这张新的手机截图"
```

或者指定 Session ID：

```bash
codex exec resume <SESSION_ID> --image new_screen.png "继续上次的问题，分析这张新截图"
```

官方命令参考里写明，`codex exec resume` 的 `--image, -i` 可以把图片附加到 follow-up prompt。([OpenAI开发者][2])

---

## 5. 不要只输入图片路径

这个区别很关键。

不推荐只这样写：

```text
请分析 ./screenshot.png
```

因为这可能只是把路径当成普通文本。Codex 也许能通过工具读文件，也许不能稳定作为“视觉输入”处理。

更稳的是：

```bash
codex -i ./screenshot.png "请分析这张截图"
```

或者在非交互恢复会话时：

```bash
codex exec resume --last -i ./screenshot.png "继续分析这个新截图"
```

如果在交互式会话里要指向项目文件，可以用 `/mention` 把文件加入对话；官方文档说 `/mention path` 会把指定文件加入 conversation，确保后续回合直接参考它。([OpenAI开发者][3])
但对于图片视觉理解，我仍然建议优先使用 `-i / --image` 或直接粘贴图片，因为官方的图片输入文档就是这样写的。([OpenAI开发者][1])

---

## 6. 和 Codex 桌面端 / VS Code 的区别

**Codex 桌面端、VS Code 插件** 是 GUI 环境，拖图、粘贴图、选择文件通常更直观。

**Codex CLI** 是终端环境，也支持图片，但它更依赖：

```text
终端是否支持图片粘贴
图片路径是否正确
当前工作目录是否正确
sandbox 是否允许读取该图片
模型是否是支持视觉输入的云端模型
```

所以 CLI 里最稳的方式不是“复制图片粘贴”，而是：

```bash
codex -i 图片路径 "你的问题"
```

---

## 7. 你的 ADB 截图场景怎么用最稳

如果你是 Android / 无线 ADB 调试，建议这样：

```bash
adb exec-out screencap -p > phone_screen.png
```

然后：

```bash
codex -i phone_screen.png "请分析这张 Android 手机截图，识别页面文字、按钮、当前状态和可能的 bug"
```

如果你已经在一个旧 Session 里分析项目了：

```bash
codex exec resume --last -i phone_screen.png "这是新的手机截图。请基于上次会话里的项目上下文继续分析"
```

或者：

```bash
codex exec resume <SESSION_ID> -i phone_screen.png "继续上次的无线 ADB 调试，根据这张截图判断当前页面状态"
```

---

## 8. 图片上下文能不能长期保留？

能进入会话上下文，但不要把它理解成“图片细节永久完整保存”。

Codex resume 会恢复之前的 transcript，让它可以继续使用旧上下文；但长会话仍然会受上下文窗口限制。官方 slash command 文档里也有 `/compact`，作用是把长对话总结，释放上下文。([OpenAI开发者][4])

所以我建议你每次让 Codex 看完关键截图后，立刻让它生成一份文字摘要：

```text
请把这张截图的信息转成后续调试用的结构化上下文：
1. 页面标题
2. 可见文字
3. 按钮和控件
4. 错误提示
5. 当前页面状态
6. 你推断出的下一步
后续请以这份摘要作为图片上下文。
```

这样即使后面图片细节被压缩，文字摘要还会更稳定地保留在会话里。

---

一句话总结：

**Codex CLI 支持图片阅读理解。最稳用法是 `codex -i screenshot.png "问题"` 或 `codex --image img1.png,img2.jpg "问题"`；在恢复旧会话时用 `codex exec resume --last -i new.png "继续分析"`。交互式里也可以粘贴图片，但成功率取决于终端环境。**

[1]: https://developers.openai.com/codex/cli/features "Features – Codex CLI | OpenAI Developers"
[2]: https://developers.openai.com/codex/cli/reference "Command line options – Codex CLI | OpenAI Developers"
[3]: https://developers.openai.com/codex/cli/slash-commands?utm_source=chatgpt.com "Slash commands in Codex CLI"
[4]: https://developers.openai.com/codex/cli/slash-commands "Slash commands in Codex CLI | OpenAI Developers"


是的，**Codex CLI 支持图片阅读理解**。官方文档现在明确写了：你可以把截图、设计图、报错截图等作为 **image inputs** 附到 prompt 里，让 Codex 和文本指令一起理解图片内容。Codex CLI 页面也把 “Image inputs” 描述为“Attach screenshots or design specs so Codex reads them alongside your prompt”。([OpenAI开发者][1])

最核心的结论是：

```text
Codex CLI 可以看图；
但图片必须明确“附加”到这次提问里；
不是把图片路径当普通文字发给它就一定能看。
```

---

## 1. 交互式 Codex CLI 里怎么看图

启动：

```bash
codex
```

进入 TUI 之后，官方文档说可以把图片直接粘贴到 interactive composer 里。也就是说，你可以复制一张截图，然后在 Codex 输入框里粘贴，再配合文字提问。([OpenAI开发者][1])

例如：

```text
请阅读这张 Android 截图，告诉我页面里有哪些文字、按钮、错误提示，以及你判断当前处于什么状态。
```

然后粘贴图片发送。

如果粘贴不稳定，我更建议用命令行 `-i` 或 `--image` 传文件，因为这个路径更清楚、更容易排查。

---

## 2. 用命令行参数传图片，最稳

官方示例是：

```bash
codex -i screenshot.png "Explain this error"
```

也可以一次传多张图片：

```bash
codex --image img1.png,img2.jpg "Summarize these diagrams"
```

官方文档说明 `-i` / `--image` 可以把一张或多张图片附到初始 prompt，多个路径可以用逗号分隔，也可以重复使用该 flag。([OpenAI开发者][2])

中文示例：

```bash
codex -i ./phone_screen.png "请分析这张手机截图，告诉我当前页面的文字、按钮、布局和可能的问题"
```

或者：

```bash
codex --image ./before.png,./after.png "请比较这两张截图，说明 UI 有哪些变化"
```

---

## 3. 支持哪些图片格式？

官方文档明确提到常见格式，例如：

```text
PNG
JPEG / JPG
```

也就是说，手机截图、网页截图、UI mockup、设计稿导出的 PNG/JPG 通常都可以。([OpenAI开发者][1])

---

## 4. 在同一个 Session 里继续看新图片

如果你已经有一个 Codex 会话，并且想在恢复同一个会话后继续传图片，可以用 `codex exec resume` 的图片参数。

官方命令参考里写到，`codex exec resume` 的 `--image, -i` 可以把一张或多张图片附加到 follow-up prompt。([OpenAI开发者][2])

例如恢复最近会话并附加新截图：

```bash
codex exec resume --last -i ./new_screen.png "继续刚才的调试。现在这是新的手机截图，请判断页面状态是否符合预期。"
```

或者指定会话 ID：

```bash
codex exec resume 7f9f9a2e-1b3c-4c7a-9b0e-xxxxxxxxxxxx \
  -i ./new_screen.png \
  "基于这个会话之前的上下文，分析这张新截图。"
```

这样图片会作为这次 follow-up 的上下文进入同一个会话。

---

## 5. 和 `@文件路径` 的区别

在 Codex CLI 里，`@` 更偏向于从 workspace 里快速提到某个文件路径，官方文档说输入 `@` 可以打开 workspace 文件模糊搜索并把路径放进消息里。([OpenAI开发者][1])

但是对**图片阅读理解**，我建议你优先用：

```bash
-i ./image.png
```

或者：

```bash
--image ./image.png
```

不要只写：

```text
请看 ./image.png
```

因为那可能只是把路径当作普通文本，是否真的作为图片附件处理就不够明确。

---

## 6. 用 ADB 截图给 Codex CLI 看

你可以这样做：

### Windows PowerShell

```powershell
adb exec-out screencap -p > .\phone_screen.png
codex -i .\phone_screen.png "请分析这张 Android 手机截图，告诉我当前页面的文字、按钮、状态和可能的问题"
```

### macOS / Linux / WSL

```bash
adb exec-out screencap -p > ./phone_screen.png
codex -i ./phone_screen.png "请分析这张 Android 手机截图，告诉我当前页面的文字、按钮、状态和可能的问题"
```

这个流程比“复制图片再粘贴到终端”更稳定。

---

## 7. 图片会不会进入会话上下文？

会。你把图片通过 `-i` / `--image` 或 composer 粘贴附加进去之后，Codex 会在这次对话中基于图片内容回答。Codex CLI 还会本地保存 transcript，恢复会话时可以继续利用原来的对话、计划历史和审批记录。官方文档说恢复会话会保留 original transcript、plan history 和 approvals。([OpenAI开发者][1])

但要注意：**图片的细节不应该完全依赖长会话自动记忆**。比较稳的做法是让 Codex 先把图片内容转成文字摘要：

```text
请先阅读这张截图，并生成一个后续调试可用的结构化摘要：
1. 页面标题
2. 可见文字
3. 按钮
4. 输入框
5. 错误提示
6. 页面状态判断
7. 和预期不一致的地方
```

这样后面即使会话变长，图片信息也更容易作为文字上下文保留下来。

---

一句话总结：

```text
Codex CLI 支持图片阅读理解。
最稳方式是：codex -i 图片路径 "你的问题"
交互式模式也支持粘贴图片，但如果粘贴不成功，优先改用 -i / --image。
```

[1]: https://developers.openai.com/codex/cli/features "Features – Codex CLI | OpenAI Developers"
[2]: https://developers.openai.com/codex/cli/reference "Command line options – Codex CLI | OpenAI Developers"
