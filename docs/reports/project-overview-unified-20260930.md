# 项目首页跨端统一交付记录

日期：2026-09-30。此记录仅说明本批证据，不替代产品或设计规范。

## 本批实现

- 主项目官方介绍由服务端统一净化并返回；PC 移除内置文案覆盖。子项目保留自己的工作区和节点快照，不继承主项目能力。
- APK 与 Windows 保留项目身份和项目图标；Windows 显示当前身份；访客了解加入方式，只读成员查看开发进度，成员继续开发；管理员和所有者从现有成员页管理与邀请成员。
- APK 首页按项目身份、继续工作、团队协作、核心能力、使用详情、资料与版本排列；动态独立成区，安装和应用图片移到介绍之后。
- 移除 APK 商店式的无来源评分与年龄展示，合并重复“关于此应用”介绍；保留项目简介编辑、图片编辑、安装、文档、成员、帖子和频道能力。
- APK 项目资源使用服务端真实链接，空数据明确说明；开发、讨论、构建按钮交给现有频道控制器，不创建任务、不扩大权限。
- 移动 Web 同步内容顺序、资料链接、动态标题及触控高度，隐藏无来源的评分与年龄；安装入口在刷新时保持唯一。
- 在现有 View 宿主内完成渐进调整，复用 V2 语义颜色，未引入新 UI 技术栈。尚未增加草稿或滚动位置的跨设备同步。

## 验证

| 范围 | 证据 |
|---|---|
| Rust | `validate-rust.ps1 -- test --manifest-path server/Cargo.toml --bin elon-server project_landing`：8 项通过，包含官方内容与子项目隔离 |
| Windows | 生产构建、严格类型检查、lint、`test:project-home` 通过 |
| 浏览器 | `test-project-introduction-browser.mjs`：主/子/空介绍、4 种加入方式、成员/访客/只读身份、成员和频道导航、折叠详情、320/390/768/1280 宽度通过 |
| PWA | 源码检查通过；真实 HTML 和生产渲染器在 320/360/411 宽度通过，无无来源指标，安装热区至少 48px；链接净化、重复刷新和清空通过 |
| APK | 最终 `:app:assembleDebug` 通过；调试包 SHA-256：`5612bcfb89ea84dcdc0883b18b77af6876bfd0bf188cfc4368b3534ec75b98fd` |
| V2 | 12 项规范检查自测及治理检查通过；不代表原生视觉通过 |

初次 Rust 命令误用不存在的 library target，已改用 `--bin elon-server` 并通过。初次 APK 编译发现 View drawable 内颜色变量遮蔽和预览回调签名问题，已修复，并完成最终构建。

## 运行与发布边界

- 原生工作台已导入任务 `project-home-unified-20260930`。模拟器自动准备超时；设备列表为空，关闭自动启动后的准备返回没有空闲模拟器。未使用物理设备，未绕过工作台手工启动或安装。
- 原生截图、150%/200% 字体、读屏、触控交互、恢复及性能尚未验证。APK 尚未正式发布。依据 apk-ui：“系统级 UI 先内部构建、关键原生运行验证，再受控正式发布。”
- 第一轮服务端发布因旧 Cargo 全局 build-dir 仍指向已满的 C 盘失败，线上未替换。重试通过仅本次进程的 `CARGO_BUILD_BUILD_DIR` 指向发布脚本选择的 D 盘缓存；不修改发布脚本或全局配置。
- 第二轮服务端编译成功；前端安装因本任务 Vite 预览占用 rolldown 依赖而失败，pnpm 回退产生非锁定依赖并触发 TypeScript 错误。已关闭本任务预览，把本次发布生成的 pnpm lock 移到临时诊断目录，以 npm 锁文件恢复依赖。本次进程的 npm 缓存和临时目录使用任务 D 盘目录，不修改系统环境。
- 共享缓存工具仅回收已预演的未锁定缓存，约 574 MB；外部缓存、活动分区和其他任务源码保留。失败发布的隔离缓存已登记退休；工具不允许清除该目录，未绕过限制。
- 功能登记 MCP 未提供，未手工编辑注册表。

第三轮正式发布成功：服务端 `0.3.1805`、服务端和 PC 前端 SHA 均为 `594453f972644775484d01a1c7db7f32bccd32fc`，PC 标记 `releaseMode=server_bundle` 且兼容服务端 SHA 一致。线上健康检查、Windows 页面资源和移动 Web 均返回 200；Windows 实际脚本含访客加入说明与管理邀请入口。

移动 Web 静态资源已随服务端更新，但 `/web` 优先使用旧的独立 runtime 模板，该模板没有新增介绍调用；完整移动 Web 首页尚未上线。未借用低风险快速通道发布本批系统级移动首页改造，待原生运行验证后再受控发布移动模板与 APK。服务端/Windows 发布成功不等于移动端已经更新。

生产功能提交为 `b9f204277`、`3f5c17529`，测试记录提交为 `594453f97`，APK 项目图标补齐提交为 `7a1b1f1b7`。最后两者不改变已发布的服务端或 Windows 源码。最终 APK 构建、工件保存完成；UI 工作台在最终源码提交后仍为 `BLOCKED / PREPARATION_REQUIRED`，不存在平台能力升级任务。

统一收尾按服务端/Windows 批次执行；APK 和完整移动模板仍列为待运行验收与发布，不能宣称本请求双端完整上线。

## UI 工作台状态

`FIT_RUN_STATUS=NOT_RUN`；`FINAL_VISUAL_LOSS=NOT_MEASURED`；`VISUAL_ACCEPTANCE_THRESHOLD=NOT_EVALUATED`；`CROSS_PLATFORM_VISUAL_PARITY=NOT_VERIFIED`；`BUSINESS_DELIVERY_READY=false`；`PLATFORM_EVOLUTION_PENDING=false`；`EVOLUTION_THREAD=NONE`；`REAL_DEVICE_STATUS=NOT_REQUESTED`；`ANDROID_RENDERER=UNAVAILABLE`。
