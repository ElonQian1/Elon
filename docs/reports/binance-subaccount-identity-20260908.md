---
version_status: current
reviewed_at: 2026-09-08
implementation_status: in_progress
---

# 币安主子账户研究隔离与身份响应证据

用户当前安排为 Win 子账户研究、Android 主账户验收。观察会话 `bb84db3f50349d0c2e2f8df091a49e44eb0bd002a931af51fd055ffea90ead83` 是用户切换后新建的 Win 会话，沿用现有站点 Profile，没有关闭旧窗口、复制 Cookie、切换手机账号或执行交易。

## 实际观察

| 路径 / 方法 | 业务结果 | 有限结论 |
|---|---|---|
| `POST /bapi/futures/v2/private/future/grid/query-open-grids` | HTTP 200、`code=000000`、`success=true`、`data=[]` | 本次列表为空，不推断全部历史网格为空 |
| `GET /bapi/accounts/v1/private/account/user/userInfo` | HTTP 200、业务成功 | `data.userId`、`data.parent` 为数字字符串；不能把 parent 当成当前 UID |
| `GET /bapi/accounts/v1/private/account/get-user-base-info` | HTTP 200、业务成功 | `data.userId` 为数字字符串、`subUser=true`、`parentUser=false`，独立确认子账户身份 |

网格列表响应资源 SHA-256 `db3bd39a7d6225853ff8d6f2f1f79542578ed2b634ef0a8a8e9c0bab00fc9ecf`。
用户信息资源 SHA-256 `6707a4b5a0fe08d317d9f5f46713d95f8dd37abc37239bd580ca3bcdbfdca395`。
基础账号资源 SHA-256 `f9ced80a88ba599d6bc57297ee124fa90132b9c77b5229346e82f07f9242c2c3`。
哈希对应研究工具处理后的完整文本，不是原始网络字节。报告只保存字段类型和角色布尔值，不保存 UID、邮箱、电话和凭据。

## 实现依据与边界

页面适配器在接受列表或详情之前，通过上述固定的认证 GET 验证当前 UID。空列表也须有身份；非空列表中的 `rootUserId` 要与当前 UID 精确相等，禁止以父子关联放宽。身份变化、认证失败和导航均清除旧权限。未匹配的子账户网格应报告不可验证，不能猜测 UID 映射。
详情响应绑定列表代次，身份校验失败或迟到响应不进入新列表；主应用只提供列表、已观察详情、撤销，不提供任意调用或金融写入。

离线覆盖身份缺失、空列表、父子 UID 不符、迟到失败、同 ID 旧详情和身份变化。Win 响应证据不等于 Android 中主动 GET 已通过，手机需以新版本单独验收。完整分页、不同账户类型的权限差异和交易功能均未由本报告证明。
