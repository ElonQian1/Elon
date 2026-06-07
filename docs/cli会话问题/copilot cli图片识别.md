copilot cli是如何发送图片让它理解的？
你知道吗？

知道。按官方资料看，**GitHub Copilot CLI 现在可以发送图片给模型理解**，方式比 Codex CLI 更像“在输入框里附加文件”。

你用的是新版 **`copilot` 命令**，不是旧的 `gh copilot`。旧的 GitHub CLI Copilot extension 已经退役，被新的 GitHub Copilot CLI 取代了。([GitHub Docs][1])

## Copilot CLI 发送图片的几种方式

### 方式 1：在 Copilot CLI 交互界面里用 `@图片路径`

进入项目目录后启动：

```bash
copilot
```

然后在输入框里写：

```text
请根据这个 UI 图实现前端页面：@mockup.png
要求尽量还原布局、间距、颜色和字体
```

GitHub 官方 changelog 明确说过：在 Copilot CLI 里使用 `@` 提及图片，会把图片作为输入提供给模型。([The GitHub Blog][2]) 官方最佳实践里也给了类似例子：`Implement this design: @mockup.png`。([GitHub Docs][3])

### 方式 2：直接拖拽图片到 CLI 输入框

官方文档说，Copilot CLI 可以把图片作为视觉参考，你可以把图片直接拖到 CLI 输入框里。([GitHub Docs][3])

实际使用时，这个体验取决于你的终端：比如 Windows Terminal、iTerm2、Warp、Ghostty、WezTerm 等可能表现不同。**最稳的办法还是保存图片文件，然后用 `@xxx.png` 引用。**

### 方式 3：复制图片后 `Ctrl + V` 粘贴

官方最佳实践也写了，可以从剪贴板粘贴图片，快捷键是 `Ctrl + V`。([GitHub Docs][3]) Copilot CLI 命令参考里也写了：`Ctrl + V` 是 “Paste from clipboard as an attachment”，也就是作为附件粘贴。([GitHub Docs][4])

所以你的操作大概是：

```bash
copilot
```

然后截图复制到剪贴板，回到 CLI 输入框，按：

```text
Ctrl + V
```

再输入：

```text
请解释这张截图里的错误，并告诉我怎么修复
```

## 它底层是怎么“发送图片”的？

简单理解是：**不是把图片路径当文字发给模型，而是 CLI/运行时会把图片作为附件处理，然后发给支持视觉能力的模型。**

GitHub Copilot SDK 文档说明了两种图片附件形式：

一种是文件附件：给一个绝对路径，运行时读取磁盘上的图片，把它编码后发送给模型。另一种是 blob 附件：直接提供 base64 图片数据和 MIME 类型，比如 `image/png`。([GitHub Docs][5])

官方 SDK 文档还写到，文件附件会由运行时自动读取并转成 base64；如果图片太大，运行时会按模型限制自动缩放或降低质量。([GitHub Docs][5]) 支持的格式包括 JPG、PNG、GIF 等常见图片类型，官方建议 PNG 或 JPEG 效果最好。([GitHub Docs][5])

## 你可以这样用

### 看 UI 截图，让它改代码

```text
@ui.png 这是我想要的页面效果。
请检查当前 React 代码，把布局改成和图片一致。
重点注意：间距、按钮大小、圆角、字体大小、移动端适配。
```

### 看报错截图

```text
@error.png 这个报错是什么意思？
请根据当前项目代码找出原因，并给出修复方案。
```

### 看设计稿

```text
@mockup.png 请基于这个设计稿生成一个 Rust + Tauri 的前端页面结构建议。
先不要改代码，先列计划。
```

## 和桌面版/VS Code 的区别

桌面版、VS Code Copilot Chat 这种 GUI 环境天然适合拖图、粘贴图片、点按钮上传。GitHub Copilot 的 IDE 文档里也写了：可以复制图片粘贴到 chat、拖放图片，或者在 VS Code Explorer 里右键图片添加到 Chat。([GitHub Docs][6])

CLI 版现在也支持图片，但它运行在终端里，所以体验会受终端能力影响。**最可靠路径是：把图片保存为文件，然后在 Copilot CLI 里用 `@图片文件名` 引用。**

一句话总结：**Copilot CLI 支持看图；推荐用 `@mockup.png` 这种方式发送图片；`Ctrl + V` 和拖拽也支持，但终端兼容性可能不如桌面版稳定。**

[1]: https://docs.github.com/copilot/github-copilot-in-the-cli "Using the GitHub CLI Copilot extension - GitHub Docs"
[2]: https://github.blog/changelog/2025-10-03-github-copilot-cli-enhanced-model-selection-image-support-and-streamlined-ui/ "GitHub Copilot CLI: Enhanced model selection, image support, and streamlined UI - GitHub Changelog"
[3]: https://docs.github.com/copilot/how-tos/copilot-cli/cli-best-practices "Best practices for GitHub Copilot CLI - GitHub Docs"
[4]: https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference "GitHub Copilot CLI command reference - GitHub Docs"
[5]: https://docs.github.com/en/copilot/how-tos/copilot-sdk/features/image-input "Image input - GitHub Docs"
[6]: https://docs.github.com/copilot/using-github-copilot/asking-github-copilot-questions-in-your-ide "Asking GitHub Copilot questions in your IDE - GitHub Docs"


是的，你的理解方向是对的：**VS Code Copilot / Codex 桌面端能直接发图，是因为它们是 GUI 客户端；Copilot CLI 也“可以支持图片”，但它不是用同一种方式接收图片。**

我前面说 Copilot CLI 不能看图，这个说法需要更正。**新的 GitHub Copilot CLI 官方文档明确写了：可以把图片作为视觉参考传进去**，方式包括拖拽图片到 CLI 输入框、用 `Ctrl+V` 粘贴剪贴板图片，或者在 prompt 里用 `@mockup.png` 引用图片文件。([GitHub Docs][1])

---

## 为什么 VS Code / Codex 桌面端更容易成功？

因为它们有真正的图形界面。你拖进去一张图，或者点附件按钮选择图片，客户端可以直接读取图片二进制数据，然后把它作为 **image attachment** 发送给支持视觉能力的模型。

GitHub 官方文档也说明，VS Code / IDE 里的 Copilot Chat 可以复制粘贴图片、拖放图片、或者从 VS Code Explorer 右键把图片加入 Chat；它支持把截图、UI mockup、流程图、网页截图等作为图像上下文来提问。([GitHub Docs][2])

所以 VS Code Copilot / Codex 桌面端一般是：

```text
图片文件 / 剪贴板图片
        ↓
GUI 客户端读取图片内容
        ↓
作为 image attachment 发送给模型
        ↓
模型视觉理解图片
```

---

## Copilot CLI 为什么你这里像是“没有收到图片”？

因为 CLI 跑在终端里，终端本质上是**文本输入环境**。图片能不能进去，中间多了几层不稳定因素：

```text
你的图片 / 截图
        ↓
操作系统剪贴板
        ↓
终端模拟器
        ↓
Copilot CLI 输入框
        ↓
Copilot CLI 把图片转成附件
        ↓
模型视觉理解图片
```

只要其中一层没把图片交给 CLI，Copilot CLI 就真的收不到图片。比如在 macOS 上，GitHub 的 Copilot CLI issue 里就提到过：Terminal.app / iTerm2 对 `Cmd+V` 的图片粘贴会在系统层拦截，导致进程“什么都收不到”。([GitHub][3])

WSL 也容易出问题。GitHub Copilot CLI 的 issue 里有人描述过：Windows 原生终端里图片粘贴可以工作，但在 WSL 环境里从 Windows 剪贴板粘贴图片到 Copilot CLI 不工作。([GitHub][4])

Windows 原生 PowerShell / Windows Terminal 也不是 100% 稳。有人反馈从 Snipping Tool / SnagIt 复制图片后粘贴到 Copilot CLI 没有任何反应。([GitHub][5])

所以结论是：

**不是模型不能看图，而是你的 CLI 环境可能没有把图片成功传给 Copilot CLI。**

---

## Copilot CLI 到底怎么加载图片？

官方文档给了三种方式。

### 方式 1：直接引用图片文件，最稳

这是我最推荐你的方式。先把图片保存成文件，然后在 Copilot CLI 里这样写：

```text
请分析这张截图，告诉我界面里有哪些按钮、文字和异常状态：@mockup.png
```

或者：

```text
请根据这张设计图实现 Android 页面布局：@screenshots/ui.png
```

GitHub 官方例子就是：

```text
Implement this design: @mockup.png
Match the layout and spacing exactly
```

官方命令参考里也写了，`@ FILENAME` 的作用是把文件内容包含进上下文。([GitHub Docs][6])
对图片来说，新的 Copilot CLI 文档把 `@mockup.png` 作为 UI 图片参考的用法示例。([GitHub Docs][1])

---

### 方式 2：拖拽图片到 CLI 输入框

官方文档说可以 drag and drop images directly into the CLI input。([GitHub Docs][1])

但实际效果取决于你的终端。很多终端拖进去的不是图片本体，而只是路径，比如：

```text
C:\Users\you\Desktop\screenshot.png
```

这时候你最好手动改成：

```text
请分析这张图：@C:\Users\you\Desktop\screenshot.png
```

或者把图片放到当前项目目录下，用相对路径：

```text
请分析这张图：@screenshots/screenshot.png
```

---

### 方式 3：复制图片后 `Ctrl+V`

官方文档说可以用 `Ctrl+V` 从剪贴板粘贴图片。([GitHub Docs][1])

但这个方式最容易受系统和终端影响。尤其是：

```text
Windows + 原生 Windows Terminal + PowerShell/CMD
```

成功概率较高。

```text
Windows + WSL
macOS + Terminal.app / iTerm2
某些截图工具复制出来的特殊图片格式
VS Code integrated terminal
SSH / remote terminal
```

这些环境就可能失败。

所以你现在 PC 的 CLI “似乎没有收到图片”，我第一怀疑不是 Copilot 模型问题，而是下面几种情况之一：

1. 你用的是旧的 `gh copilot`，不是新的 `copilot` CLI。旧的 GitHub CLI Copilot extension 已经 retired，被新的 GitHub Copilot CLI 替代了。([GitHub Docs][7])
2. 你在 WSL 里运行 `copilot`，但图片在 Windows 剪贴板里，WSL 没有把图片交给 Linux 进程。([GitHub][4])
3. 你在 macOS Terminal / iTerm2 里粘贴图片，终端没有把图片粘贴事件传给 CLI。([GitHub][3])
4. 你只是把图片路径当普通文本发了，没有用 `@图片路径`。
5. 当前选择的模型或当前 Copilot plan / client 不支持视觉输入；GitHub 文档说明 Copilot 有多个模型，不同客户端和计划能访问的模型不同，有些模型更适合多模态输入。([GitHub Docs][8])

---

## 你应该怎么测试最准确？

不要先测剪贴板。先用**文件引用**测试。

在项目目录里创建一个图片目录：

```powershell
mkdir screenshots
```

把图片放进去，比如：

```text
screenshots/test.png
```

然后启动：

```powershell
copilot
```

在 Copilot CLI 里输入：

```text
请仔细描述这张图片里有什么文字、按钮、颜色和布局：@screenshots/test.png
```

如果它真的看到了图片，它应该能说出图片里的具体内容。
如果它只是说“我无法访问该文件”或者开始根据文件名瞎猜，那说明图片没有成功作为附件进入上下文。

---

## 如果你想用无线 ADB 截图给 Copilot CLI 看

无线 ADB 不是用来“查看 Copilot 新会话内容”的，它只是可以帮你从手机拿截图。拿到截图后，仍然要用 `@图片路径` 放进 Copilot CLI。

Windows PowerShell 可以这样：

```powershell
# 从手机截屏并保存到当前目录
adb exec-out screencap -p > .\phone_screen.png
```

然后进入 Copilot CLI：

```powershell
copilot
```

输入：

```text
请分析这张 Android 手机截图，告诉我当前页面状态、可见文字、按钮，以及可能的问题：@phone_screen.png
```

如果你在 WSL 里，可以这样：

```bash
adb exec-out screencap -p > ./phone_screen.png
copilot
```

然后：

```text
请分析这张 Android 手机截图：@./phone_screen.png
```

这比“复制截图再粘贴进 CLI”可靠很多。

---

## 会话上下文里会保留图片吗？

会保留到一定程度，但不要把它理解成“永久完整保存图片细节”。

Copilot CLI 的上下文窗口会包含你的消息、Copilot 回复、工具调用和结果等内容；官方文档说这些内容构成模型当前能参考的上下文。([GitHub Docs][9])

但是长会话会触发压缩。GitHub 文档写明，Copilot CLI 在上下文接近容量时会把旧对话压缩成结构化摘要，用来继续保持连续性；但摘要化会丢失一些细节，例如每条消息的精确措辞、完整命令输出、早期小决策。([GitHub Docs][9])

所以对于图片，我建议你这样做：

```text
请分析 @phone_screen.png，并把你看到的关键信息整理成结构化记录：
1. 页面标题
2. 可见文字
3. 按钮
4. 错误提示
5. 你判断出的 UI 状态
后续都以这个记录作为图片上下文。
```

这样它会把图片内容转成文字摘要，后续会话更容易保住上下文。

---

## 最稳工作流

你的场景我建议这样用：

```text
手机 / 模拟器截图
        ↓
adb 保存为 PNG
        ↓
Copilot CLI 用 @文件路径 引用
        ↓
让 Copilot 先把图片内容转成文字摘要
        ↓
继续让它基于这个摘要改代码 / 分析 bug
```

示例：

```powershell
adb exec-out screencap -p > .\screenshots\case_001.png
copilot
```

然后在 Copilot CLI 里：

```text
这是我手机当前页面截图：@screenshots/case_001.png

请先不要改代码，先做三件事：
1. 描述截图中的所有可见文字和 UI 元素；
2. 判断当前页面可能处于什么状态；
3. 把这些信息整理成后续调试可复用的上下文摘要。
```

再下一轮：

```text
现在请根据刚才的截图上下文，检查我的 Android/Rust/ADB 自动化代码为什么没有进入预期页面。
```

---

一句话总结：**Copilot CLI 现在官方上是支持图片的，但最可靠的方式不是直接粘贴，而是保存成 PNG/JPG 文件后用 `@图片路径` 引用。VS Code / Codex 桌面端成功率高，是因为 GUI 客户端直接处理图片附件；CLI 成功率取决于终端有没有把图片传进去。**

[1]: https://docs.github.com/copilot/how-tos/copilot-cli/cli-best-practices "Best practices for GitHub Copilot CLI - GitHub Docs"
[2]: https://docs.github.com/copilot/using-github-copilot/asking-github-copilot-questions-in-your-ide "Asking GitHub Copilot questions in your IDE - GitHub Docs"
[3]: https://github.com/github/copilot-cli/issues/3104 "macOS: Ctrl+V / Cmd+V does not paste clipboard images (terminal emulator limitation) · Issue #3104 · github/copilot-cli · GitHub"
[4]: https://github.com/github/copilot-cli/issues/1217 "[Feature Request] Support clipboard image paste in WSL from Windows · Issue #1217 · github/copilot-cli · GitHub"
[5]: https://github.com/github/copilot-cli/issues/1241 "Cannot paste image from screenshot tools into CLI terminal · Issue #1241 · github/copilot-cli · GitHub"
[6]: https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference "GitHub Copilot CLI command reference - GitHub Docs"
[7]: https://docs.github.com/copilot/github-copilot-in-the-cli?utm_source=chatgpt.com "Using the GitHub CLI Copilot extension"
[8]: https://docs.github.com/copilot/reference/ai-models/supported-models "Supported AI models in GitHub Copilot - GitHub Docs"
[9]: https://docs.github.com/en/copilot/concepts/agents/copilot-cli/context-management "Managing context in GitHub Copilot CLI - GitHub Docs"
