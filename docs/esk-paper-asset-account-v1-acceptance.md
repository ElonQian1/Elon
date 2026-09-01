# ESK Paper 资产账户 V1 验收

状态：主项目 ESK Paper 账户已实现并发布；真实发行和真实结算仍关闭。

对应需求：[`requirements/esk-paper-asset-account-v1.md`](requirements/esk-paper-asset-account-v1.md)

## 已实现范围

- 主项目新增唯一 ESK Paper 账本和版本 `281` 迁移，资产 ID 为 `esk`、展示符号为 `ESK`、精度为六位小数。
- 登录用户只能读取自己的总额、可用额和卖回申请占用额；客户端不传用户 ID，避免横向读取。
- 管理员登记采用存在用户、正数精确字符串、外部引用、全局幂等键和固定确认文本；账本表由数据库触发器禁止更新和删除。
- 用户可提交和撤销自己的卖回申请。`submitted` 立即占用可用额，`canceled` 通过追加事件释放占用额，不删除历史。
- PC 账号页和 Android 个人页都显示 ESK 总额、可用额、占用额、Paper 登记、尚未上链、未划转资金、卖回申请和撤销入口。
- 所有响应和界面都保持 `simulated=true`、`funds_moved=false`、`chain_status=not_deployed`；没有价格、法币估值、固定回购价格、固定收益或主网合约地址。

## API

| 方法 | 路径 | 权限 | V1 结果 |
|---|---|---|---|
| `GET` | `/api/me/assets/esk` | 登录用户 | 本人 ESK 资产视图 |
| `GET` | `/api/me/assets/esk/sellback-requests` | 登录用户 | 本人卖回申请列表 |
| `POST` | `/api/me/assets/esk/sellback-requests` | 登录用户 + `paper` | 提交并占用本人可用额 |
| `POST` | `/api/me/assets/esk/sellback-requests/:request_id/cancel` | 登录用户 + `paper` | 追加取消事件并释放占用额 |
| `POST` | `/api/admin/assets/esk/paper-allocations` | 平台管理员 + `paper` | 追加 Paper 登记，不转移资金 |

默认 `ESK_ASSET_MODE=disabled`，未知值失败关闭。V1 不存在 `live` 或 `mainnet` 模式。

## 运行证据

2026-09-02 在隔离 worktree 和临时数据库完成：

- 生产 `elon-server` 二进制 `cargo check` 与 `cargo build` 通过。
- 临时服务端到端运行通过：匿名读取返回 `401`；目标用户总额为 `12.500000 ESK`，另一用户为 `0.000000 ESK`；提交 `4.250000 ESK` 后可用额为 `8.250000`、占用额为 `4.250000`；取消后恢复为 `12.500000 / 0.000000`，历史状态为 `canceled`。
- 上述端到端响应同时验证 `simulated=true`、`funds_moved=false`、`chain_status=not_deployed`。
- PC 严格 TypeScript/Vite 生产构建通过；ESK 相关 ESLint 和跨端静态合同测试通过。
- Android `:app:compileDebugKotlin` 通过，包含 ViewBinding 生成和新增 Kotlin 编译。
- 真实 Edge PWA 捕获通过，PNG SHA-256 为 `a5c2d1c22faa1c8a0fdecc5cdf0bb4e2c3c85ac2ff431420fbef570c94b55828`，断言余额、Paper 和未上链文案均存在。

## 已知验证边界

- 当前远端基线的完整 Rust 测试二进制受既有 SQLite VFS 测试源码编译错误阻断；错误位于 `node_agent_compute_plugin_host/.../sqlite_vfs_policy`，不在 ESK 变更范围。ESK 生产二进制和独立临时服务端端到端链路均已实际运行。
- PC 全量 ESLint 受既有 `QuantPaperLaunch.tsx` 未使用禁用指令阻断；本次修改文件的严格 ESLint 已通过。
- UI 工作台没有空闲 Android 模拟器，Android 像素验收状态为 `VERIFICATION_DEFERRED`；它不能由 PWA 截图冒充。Android 源码编译已通过，后续模拟器可用时补跨端截图。
- 本验收不证明链上发行、真实购买、官方批准卖回或付款。进入这些阶段必须另立需求并通过法域、KYC/AML、托管、多签、安全、流动性和结算门禁。

## 子项目只读投影

量化子项目没有复制本账本。主项目现签发最长五分钟、与同次 Paper grant 绑定的版本化 ESK 只读投影；量化端验签后只显示余额、来源修订和同步时间，不提供 ESK 写操作。量化实现提交为 `3efcd23cbe8baac370bbc65ba25335763ddd6b1f`，历史 NET Paper 仓位仍不得自动改名、兑换或并入 ESK。
