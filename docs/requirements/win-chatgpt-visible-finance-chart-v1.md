---
version_status: current
reviewed_at: 2026-08-23
implementation_status: planned
---

# Win ChatGPT 可见行情折线复现 V1

## 用户问题

ChatGPT 官网回答中的行情卡包含周期、走势图和指标；一龙现有 `finance` 富内容卡已经保留标题、
价格、周期和指标，也具备折线渲染器，但 DOM 适配器没有把官网当前可见的 SVG 折线几何写入
`yilong.rich-content.v1`，所以原生聊天只能显示走势图占位，视觉完整度明显低于官网。

## 范围

- 只修改 Win/Tauri ChatGPT 可见 DOM 适配器、现有原生行情卡和相应合同测试；不修改 PWA。
- 继续复用 `chatgpt_rich_content_adapter.js`、`yilong.rich-content.v1`、Rust 清洗器、DPAPI
  快照和 `AiRichContentCard`，不建立第二套图表协议或独立测试聊天窗。
- 只读取用户当前已可见回答节点内 SVG 路径的渲染几何，采样为有界折线点；不读取 Cookie、
  Token、请求头、原始响应，不发起私有请求，也不保存厂商脚本或 CSS。
- SVG 不可读、只有 Canvas、几何异常或候选不确定时保留现有官网占位与官方页回退，不伪造数据。

## 验收标准

1. 可见且有明显横向跨度、纵向变化的 SVG 折线路径被确定性采样并写入现有
   `finance.payload.chart.points`，点数和数值范围有上限。
2. 坐标无效、水平网格线、短装饰路径、回退过多或异常 API 均失败关闭，不影响标题、价格、
   周期、指标、正文和来源。
3. Rust 清洗器、DPAPI 快照和 TypeScript 协议无需引入第二种图表结构；原生卡片使用现有折线
   渲染器显示官网可见走势，多个卡片的 SVG 渐变标识互不冲突。
4. 合成几何 fixture 覆盖有效折线、水平线、异常路径和采样上限；现有私有响应授权门禁保持为空且
   失败关闭。
5. `test:user-browser`、PC typecheck/build/lint、Win Web AI 验证与源码大小检查通过。
6. 正式发布后本机运行身份精确匹配目标 SHA，并在真实 ChatGPT 行情回答中确认：可识别 SVG 时
   原生卡显示折线；不可识别时仍稳定显示官网回退，不再出现扁平时间串或空图片卡。

## 非目标

- 不从截图反推真实价格，不把像素坐标冒充行情数值，不实现交易终端或交互式缩放。
- 不绕过 `private_response_authorizations.v1.json`，不因研究许可自动开启生产私有响应读取。
- 不复制 ChatGPT 的整站样式、品牌资源或前端包。
