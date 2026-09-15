---
version_status: current
reviewed_at: 2026-09-16
implementation_status: partial
---

# 中文合约交付证据

需求见 [中文合约兼容](binance-grid-chinese-symbols.md)。原生 34 套 195 项通过，60 项中文伪传输适配器案例通过，原 12 组适配器回归通过。真实交易未执行。

正式主 APK 1.1.1771/1771 已经官方发布入口构建、签名并发布，服务器版本、源码摘要与本机一致：

- 源码：`6b5de8ea560ade5158fb51b7bf1f1f4648017d11`
- APK SHA-256：`a530d18333312f9aa23431314fe3f313777fcb0571fbd080edab4dcb3d9da15b`
- 原签名 SHA-256：`f79567cf8a7e610e218aa4b7a1292be93a9623d9bc06a9bafbf47b030f99010c`
- 无线 `adb install -r` 返回 Success，包元数据为 1.1.1771/1771。未打开应用、导航、截图或触发交易；登录资料保留。

配套量化 0.7.75/92（源 `13f89c2aeb4bb7339d492a2c9cf76209df9c2cc6`）亦已发布装机。页面、实际中文网格与交易验收由用户完成。未改 Win 会话、云同步 Rust 投影或其他交易所；不据此宣称真实网格事件订阅已接通。
