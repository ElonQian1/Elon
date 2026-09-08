---
version_status: current
reviewed_at: 2026-09-09
implementation_status: implemented
acceptance_status: pending
---

# 币安管理页连接恢复

需求：[过期恢复](../requirements/android-binance-manage-reconnect-v1.md)。

## 问题与变更

真实手机管理页显示“连接已结束，请重新授权”，策略及读取控件禁用。
Host 过期会销毁 WebView，但旧重载入口只对非空 view 调用 loadUrl/reload，
因此已销毁时无操作；MCP 还可能错误返回 reload_started。

原生与只读 MCP 现在共用重连路径：取消旧准备、调用 Host.begin、绑定恢复后的页面
及观察回调；已有页面才再次导航，新建页面沿用 begin 的首次加载。失败返回
reload_unavailable，提交中不重连。交易记录、账号隔离、保留登录资料与人工确认未改变。

重连按钮移至状态下方，优先恢复账号与列表；未选策略时收起操作表单。
管理页按钮、文字和下拉选项采用一致的深色对比样式，保留固定语义标识。

## 验证

标准 test-binance-grid-create.ps1 已通过：49项原生测试，包括新增5项重连顺序、
销毁后重建、失败与提交中阻断回归；原有 JavaScript 创建、管理及诊断测试通过。
未执行任何真实交易请求。正式发布、设备身份及本机只读恢复验收待追加。

量化 APK 的卡片与连接体验由独立子仓交付，见子仓
docs/delivery/binance-grid-usability-v1-20260909.md。不将合成视觉预览作为真实读取证明。
