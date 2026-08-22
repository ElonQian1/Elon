---
version_status: current
reviewed_at: 2026-08-23
implementation_status: in_progress
---

# Win 网页 AI 结构化富内容 V1

## 用户问题

Win 原生聊天已经能够发送 ChatGPT 与 Google AI Mode 问题，但把官网回答压平为 Markdown、
通用列表或无内容图片占位后，会丢失行情图、天气、来源标记、图片、地图和交互卡片的结构，
离线快照与手机端也无法稳定复现。每次启动若先等待官网同步，又会使输入与历史首帧变慢。

## 范围

- 只改 Win/Tauri 的生产聊天链路，不改变 PWA；Android 共享适配器保持兼容。
- 在线回答以同一官方 WebView2 中最后一条完成回答节点为视觉真值，保留官网 DOM、样式与交互；
  一龙导航、会话目录和输入框仍由原生 UI 提供。
- 离线、启动首帧与手机端使用版本化 `yilong.rich-content.v1` AST。AST 只保存用户已可见且经
  Rust 白名单清洗的语义，不复制官网 CSS/脚本，也不伪造图表数据。
- 统一覆盖正文、行内引用、来源组、图片/媒体、行情、天气、地图与未知富卡降级；继续复用已有
  来源链接、favicon、内部标签和系统浏览器组件，禁止建立第三套渲染实现。
- 可研究厂商结构化响应，但生产观察器必须匹配仓库内逐厂商授权清单。授权至少登记产品、域名、
  端点、数据类别、保存/上传范围、用户同意、保留期、到期日和撤销方式；不得读取密码。
- 原始 Cookie、Authorization、Access Token、请求头与 raw response 不进入统一消息协议、持久快照、
  日志、fixture 或云端。研究 fixture 必须脱敏、裁剪并标注来源与 schema 版本。

## 验收标准

1. ChatGPT/Google 在线完成回答优先原样显示官网最后一条回答区域，发送后不自动切换完整官方页；
   页面未就绪时立即回显原生缓存，不阻塞输入框。
2. RichContent AST 有严格版本、kind/source 白名单、长度/数量上限和未知字段丢弃；损坏、超限或
   未知版本静默降级为已清洗正文与来源，不让整条回答消失。
3. ChatGPT 行情与 Google 天气至少有真实结构化 fixture、解析器、Rust 清洗器和原生卡片测试；
   引用、来源 Logo 与来源组复用现有组件并能安全降级为域名字母图标。
4. 新增结构化响应映射入口与授权清单合同；未授权端点、过期授权、超限 body、Cookie/token/header
   字段和未知 schema 必须失败关闭，默认生产构建不得主动发起厂商私有请求。
5. 缓存仅持久化清洗后的 AST；同一 AST 可由 DOM 适配器或经授权的结构化响应映射器产生，
   Win 原生 UI 与后续手机端不依赖厂商私有字段直接渲染。
6. `test:user-browser`、PC typecheck/build/lint、Win Web AI 全量验证和相关 Rust 测试通过；
   正式发布后本机 `/api/status.release_identity` 精确匹配目标 SHA，桌面自动重开且首页不黑屏。

## 当前生产授权状态

`private_response_authorizations.v1.json` 当前为空，所以正式版只启用官网 DOM 与清洗后缓存两种来源。
厂商后续提供可审计授权时，应新增独立条目并经过评审；不得仅凭研究许可、口头说明或通配端点开启。

## 非目标

- 不复制 ChatGPT/Google 整站前端代码、品牌 CSS 或未授权资产。
- 不实现完整浏览器、扩展、密码、书签、下载管理或无限标签页。
- 不把研究许可、口头许可或本机可读取状态自动升级为生产数据授权。
