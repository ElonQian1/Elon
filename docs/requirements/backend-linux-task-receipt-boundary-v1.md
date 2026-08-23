# 后端 Linux 任务回执边界 v1

## 背景

`elon-external-pool-adapter-session-core` 是明确限定于 Linux x86_64 的协议核心，但跨平台编译路径无条件导入了其中的 `ExternalPoolAdapterTaskProtocolHostReceipt`。这会使 Windows 正式编译在 `E0432` 处失败，阻塞 Win 节点发布。

## 范围

- 保留跨平台可用的封闭语义验证 trait。
- 仅将依赖 Linux session core 的 HostReceipt 交换包装器、导出和持久化入口限制在 Linux x86_64。
- 不改变 Linux 生产协议、账本结构、鉴权、网络边界或 PWA。

## 验收标准

1. Windows 构建不再解析 Linux-only session core 的 HostReceipt 类型。
2. Linux x86_64 仍保留 `VerifiedExternalPoolAdapterBrokerTaskExchange` 的构造、导出和回执入库路径。
3. 跨平台语义验证 trait 仍可被存储层的纯语义校验代码引用。
4. 既有任务协议源码契约测试继续通过。
