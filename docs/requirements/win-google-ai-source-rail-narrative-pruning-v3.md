---
status: current
reviewed_at: 2026-08-23
---

# Win Google AI 来源栏正文分层 V3

## 用户问题

Google AI Mode 的当前完整回答根同时包含主回答与来源结果栏。一龙已经能够选中完整回答根，
但共享富内容解析器仍把来源结果栏当作普通列表追加到 Markdown，导致正文末尾重复出现
YouTube、Yahoo、新闻卡片摘要以及 `Table_content` 等来源网页片段；同一批链接又会在现有
“来源”面板中再次展示。

## 范围

- 只修改 ChatGPT / Google 网页 AI 共用链路中的 Google 可见 DOM 语义解析与对应 Win 验证；
  不修改 PWA，不复制 Google 官方 UI，也不新增聊天或来源组件。
- 继续使用当前完整回答候选、`google_web_rich_content.js`、引用部件、`AiSourceLinks` 与
  Markdown 渲染器。
- 在共享富内容解析阶段识别由多个紧凑外部链接结果组成的来源列表，把它从正文块中移除；
  引用提取仍从完整回答根执行，因此来源 URL、站点标识和来源面板保持可用。
- 主回答中的编号/项目列表、真实表格、代码、标题、段落及稀疏行内引用必须继续保留。
- 只读取用户当前可见的 DOM 结构，不读取 Cookie、Token、Authorization、请求头或私有响应。

## 验收标准

1. 至少两个由外部链接主导的来源结果项组成的列表被标记为来源集合，不进入回答 Markdown。
2. 正文列表即使包含少量行内引用，也不被误删；真实正文表格仍输出标准 Markdown 表格。
3. 被剔除的来源列表中的公开链接继续作为结构化 citation 部件进入现有来源面板。
4. 来源网页摘要中的 `Table_content` 不再污染原生正文；缺少结构证据时失败关闭并保留内容。
5. Win 与 Android 继续复用同一 Google 富内容资产和版本，不建立桌面分叉解析器。
6. Google 富内容/候选策略/消息提取合同、PC user-browser、typecheck、lint、build 与 Win 验证通过，
   正式发布后本机运行身份精确匹配目标 Git SHA。

## 非目标

- 不猜测正文中的网站名称，不伪造引用 URL、表格、图表或来源 Logo。
- 不登记或启用厂商私有接口生产授权；该路线继续由独立授权清单失败关闭。
- 不改变官方页标签、PWA、Android UI 或一龙聊天布局。
