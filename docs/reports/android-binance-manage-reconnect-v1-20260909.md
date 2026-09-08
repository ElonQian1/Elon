---
version_status: current
reviewed_at: 2026-09-09
implementation_status: implemented
acceptance_status: functional_verified_visual_deferred
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
未执行任何真实交易请求。正式发布、设备身份及本机只读恢复结果见下方记录。

量化 APK 的卡片与连接体验由独立子仓交付，见子仓
docs/delivery/binance-grid-usability-v1-20260909.md。不将合成视觉预览作为真实读取证明。

## 发布及手机只读验收

正式入口已发布1.1.1580(1580)，源码a5ba933e5d34bb36d20b1dc145f034b2f39b520f。
线上version.json与安装包SHA-256一致：
`939681aa6774bab43b48d769b88efe5771f8513607c483c6413614e6a97cce6e`。
无线ADB覆盖安装成功，设备包版本再次确认1580；原网站登录资料保留。

新版管理页原生/MCP读同一真实子账户列表，1条策略。页面上方恢复按钮可见，未选择
策略前收起操作选择器。MCP reload返回reload_started，重新选择索引0并read后，
read_outcome=verified、detail_current=true、operation_phase=idle、trading_enabled=false。
初次无Host时已成功重建页面；15分钟过期销毁分支由新增单元测试覆盖，未声称等待了
一次完整15分钟真机到期。没有提交、修改或结束真实网格，没有清除本机交易记录。

量化0.5.1原签名包已安装，真实网格卡片、列表直达与详情读取通过；视觉截图仅来自
同生产View的合成Debug预览，正式私人页面FLAG_SECURE保持不变。工作台Runtime未连接，
自动视觉门禁延期，不声明FitRun或APK/Web像素验收通过。

首次官方发布构建遇JDK loopback连接故障，在本任务设置短临时socket目录后由同一
官方入口重试成功；未修改发布程序或系统网络设置。发布脚本报告worktree自动清理
属性异常，本地收尾由已绑定任务合同的finish-ai-task另行核验。
