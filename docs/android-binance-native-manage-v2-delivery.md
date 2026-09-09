---
version_status: current
reviewed_at: 2026-09-10
---

# 币安原生管理会话命令 V2

本批实现 `android-exchange-session-host-v2` 管理切片，量化持有完整表单与最终确认，主 APK 仅新增固定命令服务。

- `BinanceManageCommands` 通过已认证 Provider 提供能力、打开、轮询、详情、准备、单次提交、取消准备、结束本机记录和关闭流程。没有主产品 Activity 启动。
- `BinanceManageCommandDraft` 只接受 settings/close/investment/range 四类严格字段；复用现有管理规则、状态机和 JS 适配器，没有新编造的私有地址。
- 账号当前读取授权是访问前提；短时单次许可另外绑定 operation、账号、文档和规范化摘要。与创建共用会话槽和恢复日志隔离；未知不重发，换账号不返回旧操作投影。
- 登录调用方只为主登录 Activity 增加量化 NativeGridManageActivity，未扩大旧主管理页来源。
- 92项管理/创建/宿主定向 Release 单元测试通过；量化171项Release、10项MCP及65项原行情前端测试通过。双方精确合同见[量化原生管理V2](https://github.com/ElonQian1/yilong-quant/blob/main/docs/contracts/binance-native-manage-v2.md)。

发布、正式安装、冷启动、真实只读和用户金融提交分别取证，不能以源码/单测代替真机。独立止盈止损编辑、追踪高级设置、完整多交易所执行和旧Activity退役尚未完成。量化全仓Rust检查在libsqlite3-sys构建脚本发生Windows STATUS_ACCESS_VIOLATION，本批没有Rust改动。

旧管理Activity仅兼容旧客户端和在途恢复。新量化默认管理路径留在子APK，能力不足显示升级/连接原因，不静默回退主表单。
