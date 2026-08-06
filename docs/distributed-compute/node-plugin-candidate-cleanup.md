# 节点插件失败候选清理

## 1. 状态

当前状态为 `partial_implementation_compiled`。Windows 受管文件系统已经具备同句柄删除原语，候选下载校验链也会保留可删除目录与文件 custody；生产 Host 尚未接入清理入口，SQLite cleanup authorization、完成回执、目录树执行器和跨重启恢复仍未实现。

本文只维护失败候选清理边界。候选本机真源见 `node-plugin-local-authority.md`，健康失败与 quarantine 见 `node-ready-capability.md`，staging 物化见 `node-plugin-archive-extraction.md`。

## 2. 已实现的底层能力

1. 新建受管文件和目录在创建时取得 `DELETE` 权限，但不会因此自动删除。
2. 既有候选 downloads 目录只在最终目录分量上取得删除权，中间前缀继续使用普通 traverse 权限。
3. 原始候选文件以父句柄相对、数据只读、share-none 且携带 `DELETE` 权限的句柄打开。
4. 删除调用 Windows `FileDispositionInfoEx`，绑定已经固定的对象，不重新解析完整路径，也不调用 `remove_dir_all`。
5. 文件删除成功后消费文件句柄；失败则返回错误和同一文件 custody。
6. 目录删除成功后消费目录句柄；非空、共享冲突或权限错误时返回错误和同一目录 custody。
7. 非 Windows 平台继续失败关闭，尚未提供 portable beneath/no-follow 删除实现。

`PinnedComputePluginCandidateArtifactSet` 现在保留完整 `PinnedComputePluginCandidateDownloads`，而不只保留根锁 lease。因此 verified、staged、健康评估和 quarantine 链没有提前丢失 downloads 目录句柄；新建 staging 文件与目录也天然保留删除权。

## 3. 当前能力不代表什么

底层句柄拥有删除权，不等于业务层已经授权删除。以下对象都不能单独触发清理：

- verification verified/rejected outcome；
- staging receipt；
- health receipt；
- quarantine receipt；
- `failed` slot phase；
- 调用方传入的布尔值、路径或 candidate token。

当前生产路径不会自动删除失败候选，也不会释放 candidate owner、清空 candidate pointer、创建新候选、恢复下载或允许重试。

## 4. 后续必须保持的事务顺序

完整清理必须拆成以下五个边界：

1. **Fresh authorization**：在 `BEGIN IMMEDIATE` 内重读 failed slot、quarantine receipt、candidate owner、inventory/state/authority/process fence 和可信时间，写入不可变 cleanup authorization，并把 owner 推进到 `cleanup_pending`。
2. **Physical execution**：只消费授权对象与保留句柄，按文件、seal、最深目录、staging run、staging 父目录、downloads 目录、candidate 根目录的顺序执行。
3. **Partial failure custody**：任何中途失败都返回已完成步骤和剩余句柄；已删除对象不能伪装成未执行，未删除对象不能按路径盲重开。
4. **Completion Store**：只有物理执行完整成功后，才能用不可变完成证据把 owner 从 `cleanup_pending` 推进到 `cleaned`、移除 failed slot、清空 candidate pointer，并精确推进 inventory/state/authority fence。
5. **Retry gate**：新候选或旧计划重试只能基于 durable completion outcome；cleanup authorization、quarantine 或内存执行成功都不能代替完成回执。

如果 completion Store 结果不确定，调用方必须只凭 recovery key 查询 `NotCreated` 或 exact `Completed`，不能重复删除、重复释放 owner 或直接开始新候选。

## 5. 目录清理规则

- 清理过程必须持续持有 `ComputePluginRootLockLease`。
- 最终对象必须来自已固定父句柄的相对打开或原 create-new 句柄。
- 目录删除前必须已经消费全部子文件和子目录 custody。
- unexpected entry 会让目录删除失败关闭，不得递归跨越未知 reparse point。
- 路径字符串只用于规范逻辑名称、审计和错误信息，不参与授权性删除 lookup。
- 权限、共享冲突或非空错误不允许降级到普通路径 API。

## 6. 当前验证

`elon-pc-node` 已通过编译和 3 项 Windows 定向测试：

1. 以 create-new 句柄删除精确文件，再删除目录；
2. 非空目录删除失败后保留原目录 custody，删除子文件后用同一 custody 重试；
3. 重新固定既有文件的 cleanup 句柄并完成文件与目录删除。

这些测试证明底层 Windows 语义，不证明 cleanup authority、SQLite 完成事务、崩溃恢复或生产 Host 接线。

## 7. 下一批实现边界

下一批应先增加 cleanup authorization schema、typed permit 与 outcome-uncertain recovery，再实现目录树线性执行器和 completion Store。不得先暴露“按 candidate token 删除目录”的管理接口，否则会绕过 quarantine、owner 和 fence 合同。
