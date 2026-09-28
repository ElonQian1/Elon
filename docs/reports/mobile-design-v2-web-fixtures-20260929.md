---
version_status: current
decision_status: evidence
reviewed_at: 2026-09-29
implementation_status: fixture_verified_parity_pending
---

# V2 网页离线状态验收夹具

需求为 [移动设计 V2](../requirements/mobile-design-v2-adoption.md)。本批补齐主项目网页的
可重复观察环境，供后续工作台原生/网页语义验收；并修复该环境复现的生产网页布局缺陷。
认证、消息协议和金融行为保持原合同。

## 实现范围

- `scripts/mobile-design-pwa-fixture.cjs` 只监听回环地址，加载仓库中的生产 HTML、JS、CSS
  与全部正式 PNG 替换；图片映射直接读取 `web.rs` 已声明的资源，未知占位符失败关闭。
  成员和文件夹图标的既有路由也读取正式 Android 资源，不用占位图片掩盖加载失败。
  固定合成账号经测试登录接口产生进程内短期会话；不转发外部网络请求。
- `scripts/mobile-design-pwa-navigation.js` 仅由夹具服务加载，点击真实可见菜单和页面控件；
  不重写 DOM 内容、样式或业务函数，不给生产 HTML 添加测试分支。
- `scripts/test-mobile-design-pwa-fixture.cjs` 通过真实浏览器登录和导航，验证必要状态及
  未认证读取、错误登录、跨来源请求、未知 API、密码/撤销会话/项目删除拒绝。
- 对话结果来自固定的单向本机 WebSocket 流，只发送已声明的离线结果，不执行消息任务。
  该有界流不是通用 WebSocket 服务，任何客户端数据均断开；不证明真实消息发送或重连。
- 夹具不提供 Service Worker，避免合成 API 被长期缓存。主题只用既有外观设置。

真实入口核查：当前 PWA 的“项目”标签进入广场，个人/群体项目从底部菜单打开；
不能按已隐藏的旧 `projectHomeRoot` 路径宣称当前项目列表通过。夹具使用现有
`bottomMenuBtn`、`projectBrowserSheet` 和项目按钮，再沿原回调进入对话。

## 已验证

真实浏览器 16 组检查通过：公开登录与注册；实际测试登录和写入拒绝；浅/深主题各自的
长中文项目列表、空列表、项目到结果、账号安全。生产脚本无未捕获错误。
账号安全呈现固定密码已启用、单个当前会话、Google 未配置；未触发绑定、密码或恢复码变更。
登录后夹具保留固定示例数据，不代表任何真实账号。未测量网页 200% 字体或原生/网页相似度。

新增 6 组浅深主题 × 320/411/720 宽度的聊天布局回归先失败、修复后通过。实际根因是
隐藏项目页仍更新全局工具栏与底部动作，且 `tabs` 没有在网格中声明，导致 411px 页面正文
仅有 275px；广场样式又通过隐藏页面匹配聊天工具栏，回落到旧黑色。现仅在项目页激活时
更新对应操作、声明底部网格区域，并将广场工具栏限制到激活页面、使用主题表面。
检查覆盖正文/工具栏全宽、无横向溢出、主题背景、无误入聊天的项目发帖按钮和可见图片加载。

## 可重复执行

在已隔离的工作区运行，使用项目日志包装器保存输出。Node 须能解析 Playwright，浏览器默认
为现有 Edge；可通过 `PLAYWRIGHT_MODULE`、`BROWSER_CHANNEL` 指定已有运行时，不自动安装。

```text
node scripts/test-mobile-design-pwa-fixture.cjs
```

正式工作台捕获前，在后台保留单个本机服务（端口仅示例，先确认未占用）：

```text
node scripts/mobile-design-pwa-fixture.cjs 43781
node scripts/test-mobile-design-pwa-fixture.cjs --origin=http://127.0.0.1:43781 --prepare-profile
```

第二条通过实际页面登录后，仅将所需本机会话写入已忽略的
`.elon/ui-tuner/pwa-sessions/mobile-v2-offline.json`；不得提交或输出会话值。
进程结束后会话失效，重启服务须重新准备。认证档案不存放业务数据或生产凭据。

| 状态 | 夹具 URL 查询 | 正式捕获就绪条件 |
|---|---|---|
| 登录 | `?fixture=login` | `#loginView`，PUBLIC_LOGIN |
| 注册 | `?fixture=register&auth=register` | `#nicknameInput`，PUBLIC_LOGIN |
| 项目及长中文 | `?fixture=projects&tab=projects` | `#projectBrowserSheet.active` 与长标题按钮 |
| 项目空状态 | `?fixture=empty&tab=projects` | `#projectBrowserBody` 的暂无个人项目 |
| 对话结果 | `?fixture=chat_result&tab=projects` | `#chatList .bubble.ai` 的离线结果 |
| 账号安全 | `?fixture=account_security&tab=profile` | `#accountIdentityMask.active` 与密码/会话结果 |

已登录状态使用 `authProfile=mobile-v2-offline`；主题使用既有 `mobile-v2-light/dark`
fixtureProfile。公开登录/注册不携带 authProfile。只向工作台传入回环 URL 与档案名称。

## 未完成及证据边界

本批只验证夹具与生产网页在该环境下的行为。正式语义 JSON 规格、完整原生/网页状态映射、
工作台截图及签名回执仍待补齐，不能由 16 组浏览器检查推导跨端门禁通过。
首次工作台截图暴露夹具只替换品牌、未替换其余 PNG 的缺口；现已修正并新增占位符断言，
旧图保留为失败发现证据，不作为完整视觉通过。账号安全页的真实密码框被捕获器误判为
登录页，返回 AUTHENTICATION_FAILED；独立浏览器检查通过，该工具缺口交原授权修复会话，
不以隐藏密码框、删除状态或伪造 DOM 绕过。公开注册的就绪容器应使用 `#loginView`，
另断言 `#loginBtn` 为“创建账号”，昵称输入框本身不构成公开登录表单容器。
规格必须保留六类状态的浅深主题和已要求的大字体原生覆盖，不能缩减为公开登录页。
主项目既有原生截图绑定 `0f097ff`；量化最新原生证明绑定 `a39b3ed`，两者不能混用。
两套 V2 APK 尚未正式发布或覆盖安装，整套页面推广及完整业务/性能验收仍未完成。
