---
version_status: current
reviewed_at: 2026-08-24
implementation_status: planned
---

# Win ChatGPT 行情 K 线富内容 V1

## 用户问题

Win 原生聊天已经能保存行情标题、价格、周期、指标和可见 SVG 折线，但不能表达股票回答中的
开盘、最高、最低、收盘（OHLC）K 线。用户需要在不登录 ChatGPT 的游客会话中也能稳定看到
官网已生成的完整回答，并在官网明确公开 OHLC 语义时显示原生 K 线。

## 范围

- 在现有 `yilong.rich-content.v1` finance 卡内增加 `candlestick` 图表分支，继续复用现有富内容
  消息、Rust 清洗、DPAPI 快照和 React 卡片，不建立第二套聊天或缓存链路。
- 只读取官方回答 DOM 中公开的 `aria-label` / SVG `title` OHLC 文本；不根据颜色、像素或截图
  反推价格，不读取凭证、Cookie、请求头或未授权私有响应。
- 保留既有 SVG 折线支持；没有至少两根合法 OHLC 数据时稳定回退到折线或官网交互提示。
- 同步提升 ChatGPT 共享适配器版本，使已安装 Win 端在升级后重新装载游客 stream-status 修复。

## 验收标准

1. finance AST 能表达 2–512 根有界 K 线；每根包含非空时间标签和有限的 open/high/low/close，
   且 high 不低于开收盘、low 不高于开收盘。
2. ChatGPT DOM 仅从公开无障碍文本提取中英文 OHLC；重复、缺字段、矛盾或未知结构失败关闭。
3. Rust 与 TypeScript 双层校验拒绝异常数据，DPAPI 快照只保存清洗后的稳定 AST。
4. Win 原生卡以红跌、绿涨、灰平绘制影线和实体，并保留可访问标题、区间与逐根 OHLC 描述。
5. 游客 stream-status、富内容合同、前端 typecheck/build/lint、Rust 测试和源码大小检查通过。
6. 正式发布安装后，Win 运行身份精确匹配发布 SHA；真实游客历史回答可同步，K 线在有官方 OHLC
   语义时显示，否则明确回退且不伪造。

## 非目标

- 不提供交易、下单、技术指标计算、缩放或逐笔行情。
- 不保证所有第三方 Canvas 图表可离线复现。
- 不绕过现有私有响应授权清单。
