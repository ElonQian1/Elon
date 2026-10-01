---
title: 工作室缓存安装与验收
version_status: current
implementation_status: partial
reviewed_at: 2026-10-01
owner: platform
---

# 工作室缓存安装与验收

本文说明已经接入工具的行为和必须在设备上执行的验收。目标架构见
[分层方案](design/studio-build-cache-v1.md)，各项目 AI 从
[统一入口](studio-cache-ai-entry.md) 开始。发布包存在不代表其他电脑已经安装。

## 可用能力与尚待接入

| 能力 | 工具行为 | 设备验收 |
|---|---|---|
| 目录归属 | UNC 和已有 owner 标记的本地路径均检查机器身份 | 宿主应使用自己的新根 |
| 本机工具 | 显式 control root 下安装平台、启动器目标、Cargo include、sccache 控制文件 | doctor 检查安装指纹和激活 |
| 空间接纳 | Cargo 包装器独立检查 build、target、TEMP；同卷合并并扣除当前用户的预约 | 不等于跨账户、跨电脑或存储侧配额 |
| 本地优先 | 启用机器 profile 后先计算本地容量，显式允许时再选独占 UNC | 网络错误直接报告；不会暗中返回 C 盘 |
| 本地对象缓存 | 独立本机 L1，bootstrap 初始上限 2 GiB | 上限不包含 target、下载包、链接临时文件 |
| sccache 共享对象 | 支持经校验的原生 disk + S3/WebDAV 配置 | 必须另有可访问的服务、凭据和跨机命中验收 |
| 宿主接入 | Host bootstrap 在本机查询真实 SMB 共享物理路径 | 在宿主执行；不能从共享名称猜盘符 |
| 代编译 | 机器 profile 可报告需要远程执行 | 自动提交、队列、隔离执行器尚未交付，不因标志为 true 而获得执行能力 |
| Gradle/包代理 | 保持各工具原生缓存边界 | 尚未部署，不共用可写 Gradle home/node_modules |

目前 control root 以外的各 build root 仍保留自己的分区注册表和 GC 策略；这是已有数据的
兼容管理结构。不要把它们移动到统一控制目录、合并不同电脑的注册表或用普通文件删除替代 GC。

## 发布包与本机安装

发布包包含 bootstrap、Rust 平台、Skill、项目清单和本文档。`studio-cache-bundle.json`
列出完整文件集合及 SHA-256。接收方先复制或解压到本机普通目录，核对随交付提供的
manifest 摘要；工具拒绝从 UNC 或重解析路径加载安装包，也拒绝额外文件、缺文件和摘要变化。
摘要通过可信交付渠道取得，不能仅相信与包放在一起的可修改说明。

从本机包目录执行预演，`<manifest-sha256>` 用交付给你的实际值替换：

```powershell
& .\scripts\initialize-studio-cache.ps1 -Role Host -ExpectedSourceSha256 <manifest-sha256>
```

核对宿主本地映射与容量后，使用相同命令加 `-Apply`。其他开发电脑使用 `-Role Client`。
bootstrap 默认预演不写文件；底层 `rust-cache.ps1 install` 本身即使没有 `-Apply` 仍会安装文件，
两者语义不同。宿主需要本机 Rust/Cargo、sccache 及能查询 SMB 共享的权限；缺失时保留诊断，
不自动放宽防火墙、安装陌生软件或猜测盘符。

客户端默认只启用本地 build root。`-AllowUncBuild` 是独占 UNC 溢出的显式选择，
只在该电脑完成兼容性测试后启用。机械盘千兆共享不默认承担 4–5 台同时构建。
宿主自己的 build root 是共享目录对应卷上的本地路径，身份后缀来自机器标识摘要。
宿主与每台客户端各占一个目录；访问 SMB 的能力不是接管其他目录的权限。

bootstrap 将无凭据 profile 和 tiers 配置写入本机，设置当前用户环境，并调用官方安装与 doctor。
执行失败会还原原路径环境；Cargo 配置和启动器只有仍匹配本次安装预期内容时才恢复备份。
并发修改会保留并报告需要处理，安装目录和缓存保留；这不是删除全部安装产物的事务回滚。
成功报告写入共享的 `studio-bootstrap-reports`，便于核对角色、卷容量和诊断结果。
安装属于当前 Windows 用户；换用户需要单独接入，不能宣称整台机器所有账户都已配置。
安装器更新当前安装进程和用户环境；已打开的其他终端、IDE、Codex 进程可能仍继承旧值。
保存工作后重新打开相关应用，再核对 `doctor` 和实际构建的路由输出。

## 路径和容量的真源

- `ELON_RUST_CACHE_CONTROL_ROOT`：本机工具、控制文件及 `capacity-v1` 预约账本。
- `ELON_RUST_CACHE_SCCACHE_TIERS`：本机对象缓存配置文件；凭据不写进 JSON。
- `ELON_STUDIO_CACHE_PROFILE`：机器身份、角色、本地候选根和经允许的共享候选根。
- `ELON_RUST_CACHE_ROOT`：默认受管根；旧入口及分区管理保持兼容。

没有 profile 的旧安装仍使用原路径规则。机器配置不进入项目 Git。
一轮构建一旦接纳就固定根目录、target 和预约；网络断线不能中途换根继续。
显式 `-CacheRoot` 仍可选择受管位置，但不能绕过归属或容量接纳。
直接绕过平台运行裸 Cargo 不经过平台预约；项目和 AI 必须使用现有构建包装器。

`status`、`doctor` 和 `gc` 等管理命令使用 profile 的默认管理根，不要求候选目录先具备
构建空间；即使所有卷都不满足新构建预算，也能继续诊断。需要检查其他已知受管根时
显式传 `-CacheRoot <absolute-root>`，不能用它跳过归属检查或把清理预演当成清理完成。

初始未测预算为 build 增长 8 GiB、target 增长 2 GiB、TEMP 增长 1 GiB；
每卷保底为 `max(10 GiB, critical_free_percent × 卷容量)`。它们是保守起点，
不是准确的项目峰值。基于实际构建峰值修改各受管根 policy 的
`capacity_build_growth_bytes`、`capacity_target_growth_bytes`、`capacity_temp_growth_bytes`
与 `capacity_floor_bytes`，不能用 0 假装不需要空间。
L1 所在卷另外保留一次其最大配额，默认 2 GiB；这部分不随每个任务重复计入预约。
它是保守余量，未扫描现有对象来抵扣；对象缓存淘汰不负责控制活动 build/target 总量。

预约账本按机器、用户、PID 和进程启动时间验证；不同项目共享同一账本，
损坏或无法确认归属的预约会阻止新构建。预约在 `finally` 中释放；父进程异常终止可能留下
仍在写盘的子进程，因此遗留预约继续占预算，需检查所有写入者后维护。它不能阻止游戏、其他账户或外部程序继续写盘。
UNC 别名对应同一物理卷的全网容量需要宿主服务或存储配额；本机账本没有这项保证。

`-SkipCacheGc` 和 `auto_gc_on_run=false` 只决定是否清理，不跳过构建接纳。
空间不够时修正任务位置、实测预算或回收已审查缓存；不删除游戏来通过门禁。

## 共享对象服务的启用门槛

tiers 文件可描述受支持的 S3/WebDAV 端点、命名空间、访问模式和本机缓存大小。
凭据从本机环境或工具原生凭据链取得，不写 Git、共享 JSON、URL 查询参数或日志。
远端必须支持 TLS；本机回环服务可使用 HTTP。配置固定到一个专属 sccache 实例，
不能连接未知的已有进程或随不同项目来回切换 daemon 配置。

sccache 0.16.0 的只读远端与本地写入组合有已知限制。适配器拒绝未经验证的只读组合，
不能把“配置被接受”当作本地回填正常。无服务时 bootstrap 使用本地 L1，明确没有 L2。

共享服务的完成标准是：同一源码/工具链，电脑 A 冷编译写入，电脑 B 本地空缓存命中远端，
电脑 B 再次构建命中本地；同时核对正确输出。随后验证服务断线、写入失败、容量耗尽、
并发同键和信任域隔离。全部通过前不要修改状态为“跨机验收通过”。
宿主尚未确认服务运行环境、权限及磁盘时，先运行 bootstrap 取得事实再选择部署方式。

## 性能与维护

受限存储探针只在已有 owner 标记的根中新建随机临时目录，并在身份复验后清除自身文件：

```powershell
& .\scripts\measure-studio-cache-storage.ps1 -CacheRoot <本机所属的受管根> -OutputPath <本机报告路径>
```

默认 64 MiB 大文件和 128 个 4 KiB 小文件；读取紧随写入，可能命中内存缓存。
结果只说明单客户端样本，不代表机械盘持续速度、冷构建或 4–5 台并发。
网络协商 1 Gbps 也不能代替端到端测试。先测大文件、小文件和实际构建，再决定 SSD/网卡升级。
官方说明见 [Microsoft SMB 性能文档](https://learn.microsoft.com/en-us/troubleshoot/windows-server/networking/slow-smb-file-transfer)。

退役发布快照使用 `maintain-node-rollback.ps1` 的新预演、精确摘要和已验证归档流程。
发布/修复进程活动或运行身份变化时暂停清理并重建计划。不要重用过期计划、删除当前版本，
或把归档记录当缓存 GC。日常只通过官方 status/doctor/gc 查看与回收。

## 交接给其他项目的 AI

项目 AGENTS 或对应入口保留一条指向本文档的受信版本/本机安装副本的链接。
Rust 项目用 `adopt-project` 生成项目身份和薄包装器；不在每个仓库硬编码共享路径。
全局 Codex Skill 只是辅助入口，不保证其他厂商代理会自动加载。
接入记录必须分别写明代码版本、测试、当前用户安装、宿主执行和跨机验收，不能合并为“已完成”。
