---
version_status: current
reviewed_at: 2026-09-29
implementation_status: verified_locally
---

# PWA 群聊输入无法激活修复

## 根因与范围

空输入栏显示的是占位按钮。真实 textarea 与其父级默认 `display: none`，
只有 `input-focused` 或 `has-text` 时显示。原点击事件对隐藏 textarea 调用
`focus()`，但 `input-focused` 又依赖 focus 事件，首次输入无法启动。
本地生产页面触摸测试在修复前明确失败：点击后 activeElement 不是 textarea。

另外，展开后的定位文本行带有底部负 margin，空白区域覆盖下方操作行，
浏览器命中测试明确报告其拦截附件按钮。操作行增加相对定位后按 DOM 顺序
绘制在预留的底部区域上层，附件与发送按钮恢复点击。

修改只涉及 `server/src/assets/web_page.html` 的输入栏点击顺序和操作行定位。
点击时先显示编辑区，再在同一用户事件内同步聚焦。群聊、私聊和项目共用修复。
不变更成员权限、消息协议或 Android 原生输入组件。

## 验证

- `scripts/test-pwa-chat-input.cjs` 使用完整生产 HTML、样式和事件；复用本地
  PWA fixture，以合成群聊、好友和消息响应验证，禁止访问外部 origin。
- Chromium/Edge 与 WebKit 各通过 6 个场景：浅/深主题 × 群聊/私聊/项目。
- 每场景覆盖首次触摸、中文输入、失焦后继续编辑、清空重输、附件打开后
  返回输入；群聊/私聊还验证单次发送正确目标、消息渲染和发送后继续输入。
- 每个引擎仅发送 4 条本地合成消息；没有生产消息或项目任务发送。
- 既有 `test-mobile-design-pwa-fixture.cjs` 通过，包括 320/411/720 宽度布局。
- 移动 V2 规则与 12 项规则自测通过；静态发布模板生成通过。
- WebKit 运行截图在 `.ai-tmp/pwa-chat-input/`，仅为桌面引擎的移动视口证据。

## 交付与验收边界

生产页面代码的本地输入回归已通过；发布使用 `scripts/publish-mobile-pwa-static.ps1` 的
原子模板替换及摘要、HTTP runtime 来源回读，以发布命令回执为最终依据。
统一收尾使用预检给出的 CodePushed 合同；独立核对 PWA 模板发布结果。

尚未验证真实 iPhone 主屏幕 PWA 的系统键盘、刘海/安全区及后台恢复。
用户需完全退出并重新打开 PWA 复核键盘与输入；不需要退出账号或清除数据。
当前会话未提供 Feature Registry MCP，未手写功能注册表。
