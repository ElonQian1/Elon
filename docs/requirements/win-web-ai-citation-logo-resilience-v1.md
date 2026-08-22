# Win 网页 AI 引用 Logo 韧性 V1

## 目标

在不改 PWA、不复制官方样式代码、不读取 Cookie 或 Authorization 的前提下，复用现有来源卡片和内联引用组件，让 ChatGPT / Google AI 的公开来源尽量显示可辨认的网站 Logo；远程图标不可达时必须立即保留稳定的站点字母标识，不能长期显示空白白圆。

## 当前问题

- 原生来源模型已经获得结构化引用 URL，但生产诊断中 `citation_logo_count=0` 很常见。
- 前端回退顺序优先请求 Google favicon 服务；该服务在部分 Windows 网络环境下会长时间挂起，导致后续站点 `/favicon.ico` 永远不执行。
- `<img>` 在远程图标完成前直接覆盖字母底图，因此挂起请求表现为空白白圆。
- ChatGPT 引用适配器只在少量关联节点中找 `img`，Google 提取器没有携带可见链接内部的公开图标。

## 实现范围

1. 继续使用 `AiSourceMark`、`AiSourceLinks` 和 Markdown 内联引用组件，不新建第三套来源 UI。
2. 图标候选顺序固定为：上游已清洗官方图标、来源站点同源 `/favicon.ico`、仅含公开 hostname 的远程 favicon 回退。
3. 每个候选设置有界加载超时；加载成功前不遮盖本地字母标识，失败或超时自动尝试下一候选。
4. ChatGPT / Google DOM 适配器只从当前可见引用控件或链接的后代节点提取 HTTPS 图标；持久 AST 仍移除 query/hash 和凭据字段。
5. 所有图标继续使用 `no-referrer`，不发送回答页地址；不缓存网页正文、URL query、Cookie、token 或 Authorization。

## 验收标准

- 来源站点 `/favicon.ico` 排在远程 favicon 服务之前，服务不可达时不阻塞本地同源候选。
- 图标未加载、加载失败或超时时，来源标记始终显示稳定字母底图，不出现空白白圆。
- 成功图标淡入且复用于来源摘要、来源卡片和正文内联引用。
- ChatGPT 与 Google 可见引用节点中的公开 HTTPS 图标能进入既有 `iconUrl` 字段；带 query/hash 的临时地址不会进入持久 AST。
- `npm run test:user-browser`、typecheck、lint、build 和 Win 验证通过；PWA 无修改。

## 非目标

- 不抓取第三方网站 HTML 来解析全站 favicon。
- 不代理、下载或持久化任意远程图标二进制。
- 不导入 ChatGPT / Google 官方 CSS、React 组件或私有凭据。
