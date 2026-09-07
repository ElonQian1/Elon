---
version_status: current
status: in_progress
reviewed_at: 2026-09-08
owner: node-update-compatibility
---

# Win 状态压缩修复与私人网格同步准备

## 已验证的代码修复

状态回复进入强压缩时，原生产端会删除更新器必需的 prompt 和 runtime 身份。
先用 `ceef70656` 将原压缩代码独立为生产模块，单项既有测试通过，再增加回归
场景复现问题：原代码 3 项通过、7 项失败。修复后全部 10 项通过。

修复保留原始 prompt 数量和完整 ID，将 runtime 条目压缩至已有 task_id；缺失、
类型错误和数量不一致仍保留为无效证据，不合成空闲状态、不截断 ID。身份整体
仍过大时回复明确降级，更新器继续拒绝。极端 bootstrap 数据也不能突破 64 KiB。
更新器、安全恢复账本、任务状态和运行中的窗口没有修改。

测试 harness 直接导入实际生产模块，未复制压缩算法或构造替代 NodeRuntime。
正式日志：

- 基线：`node-status-payload-baseline-20260908-20260908-003225-845`，exit 0。
- 复现：`node-status-identity-regression-red-20260908-20260908-003824-338`，exit 101。
- 修复：`node-status-identity-regression-green-20260908-20260908-004238-480`，exit 0，46.5 秒。
- 通过指纹：`66a903840242e2442767fcba2057928be2fd69e7c731ff035df68ec915862d7f`。

首次完整节点 check 在 num-traits 0.2.19 和 thiserror 1.0.69 的构建程序发生
`0xc0000005`，尚未验证节点源码。日志
`node-status-identity-integration-check-20260908-20260908-004406-489`，73.1 秒。
后续以进程范围的固定编译器、标准 MSVC 环境和单任务配置重检，完整节点 check
通过：`node-status-identity-pinned-integration-check-20260908-20260908-004812-762`，
exit 0，210.5 秒，指纹
`61a9cc32c0a05a9fba532da7b258a1d22167d2974c14824688561e59f5d3f319`。
这是完整非测试节点的编译检查，不是全节点历史单测或正式 release 构建。
准备脚本曾因空的 encoded rustflags 变量被指纹校验拒绝；移除该空变量后运行，
未更改验证平台、机器环境或缓存门槛。

## 同步 worker 已构建

量化源码保持 `f28f9ca6302917d48201c449a3804c4192235e49`；没有修改已签名 APK 的
源码身份。固定 rustc 1.97.0 并加载正式 MSVC 环境后，实际 worker 构建成功。
日志 `quant-grid-worker-msvc-environment-20260908-20260908-003550-209`，exit 0，
70.6 秒。此前进程异常的唯一底层原因未确认，不能仅凭成功重建归因于 MSVC。

- 程序：`yilong-grid-read-bridge.exe`，dev profile，8,263,168 字节。
- SHA-256：`044d825b63df617e7029b0499ed09490d888223fef58eadd7af3ec03a310fbb4`。
- 持久工件目录：`D:/rust/active-projects/ElonNodeData/artifacts/private-grid-read-bridge-v1/worker-f28f9ca63029`。
- `build-receipt.json` 保存源码、工具链入口与构建身份；`startup-smoke.json` 保存
  实际无参数启动的结果：exit 1 / project_required，符合在创建状态及连接前拒绝的合同。
- 未启动账号同步、创建同步 ledger 或重跑此前 17 项 worker 测试。

这补齐“没有独立可启动 exe”的工件缺口，不证明账户数据已上送或 APK 已读到。

## 运行验收边界

已安装 Win 仍为 aa687；新的读取 binding、真实云端 ACK、授权后 APK 查询、撤销
与账号切换仍待验收。20 条历史更新恢复阻塞和原子替换拒绝访问没有被本修复消除。
新的状态生产代码只有经正式升级加载后才生效，不能用于证明旧进程现场已修好。
本轮没有再次重启 Win，没有操作 Chrome，也没有发起真实交易。
