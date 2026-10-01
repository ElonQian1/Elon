---
version_status: current
decision_status: accepted
implementation_status: released
reviewed_at: 2026-10-01
---

# 群成员模块交付证据

需求见 `docs/requirements/group-members-management.md`。本报告记录交付事实，不扩大产品范围。

## 能力与实现边界

| 能力 | 交付内容 | 验证 |
|---|---|---|
| 完整名单 | 服务端成员关系、准确总数、每页 50 人、全群昵称搜索、管理员/最近加入筛选、版本游标恢复 | Rust 1000 人用例；浏览器和 APK 172 人用例 |
| APK/移动 PWA | 最多三行头像预览、完整纵向名单、加载更多、搜索、资料和管理入口 | 浏览器交互；APK 单元测试与隔离模拟器实际页面 |
| Win/宽屏 PWA | 可收起的当前群成员右栏；窄窗口使用弹层；不以全站用户替代群成员 | PC build/lint；宽窄浏览器测试；PWA 输入区不被侧栏覆盖 |
| 管理权限 | 邀请、邀请审核、最多 100 人批量移除、任免管理员、转让、退出、解散；管理动作需明确确认 | 服务端权限和事务用例；客户端交互用例 |
| 同步与安全 | 请求幂等；事务通知；群/账号隔离；失去成员权限后清空名单；轮询/重连刷新 | 游标冲突、重复请求、403、群切换、注册入群回归 |

主要模块：`server/src/store/groups/`、`server/src/router/social_routes/group_management.rs`、`pc-frontend/src/features/friends/members/`、`server/src/assets/group_roster.js`、`android/app/src/main/kotlin/com/elon/app/GroupMember*.kt`。

主业务提交 `bbfab8d6316e9049eb44b21de552c97e5566f514`；最终运行候选 `994e32ff36c6059e1e6c819bce5c958fc107904f`，包含主线消息分页和图片预览修复。迁移 309 为群管理，310 为主线消息时间线。

## 验证证据

- 后端群管理 6 项通过，包含 1000 人分页、搜索、权限、原子性、重复请求、邀请策略、转让/退出/解散。注册入群回归另行通过。
- PC 构建与 lint 通过；`test-group-roster-browser.mjs` 在合入主线后通过。浏览器测试关闭 `crypto.randomUUID`，验证普通 HTTP 环境仍能提交管理命令。
- APK 最终 5 项测试通过：`GroupMemberScreenTest` 3 项、`GroupMembersDialogTest` 2 项，含 2 倍字体初始化、172 人逐页加载和权限失效恢复。
- 隔离设备 `emulator-5554`，包 `com.elon.app.uitest_398d0d9d`，generation 4。实际查看群主深色名单、邀请窗口、普通成员浅色 2 倍字体页面；无真实群成员写操作。
- `ui_build_and_verify` 操作 `ui_build_verify_083fea73e008498886cb1a0200dd7cb3` 成功；APK SHA-256 `11fbe05eed222970ec0eaaa53ad851377aa29bc3e6fba5e78fc93665c539b0c6`。重建前后源码画面比较通过，`sourceParityLoss=0`。该数值仅代表重建一致性，不能解释为 APK/PWA 像素相似度。
- 本机命令证据：`group-members-backend-final-20261001-184746-107`、`group-members-registration-tests-20261001-185929-098`、`group-members-integrated-pc-20261001-192916-136`（其中 build/lint 通过，末尾浏览器命令因未设置依赖路径失败）、`group-members-integrated-browser-20261001-193104-325`（修正运行环境后通过）、`group-members-preview-regression-20261001-193433-416`。

## 自动视觉验收边界

微信截图作为风格参考，无干净像素目标。实际页面和功能回归已检查，但完整 APK/PWA 跨端验收工件尚未生成，不能宣称自动视觉验收全部通过。Dialog 的实际截图/文字可由窗口追踪取得，Runtime 语义树只报告预览 Activity 的 9 个基础节点，尚不足以对完整目录和管理弹窗绑定跨端能力状态。

候选 `994e32ff3` 的 `ui_check_workflow_completion`：

- `NATIVE_RUNTIME_SOURCE=PASSED`
- `FIT_RUN_STATUS=NOT_REQUIRED_WITHOUT_CLEAN_TARGET`
- `FINAL_VISUAL_LOSS=NOT_MEASURED`；`VISUAL_ACCEPTANCE_THRESHOLD=NOT_APPLICABLE`
- `CROSS_PLATFORM_VISUAL_PARITY=MISSING_OR_FAILED`
- `BUSINESS_DELIVERY_READY=false`（UI 工作台门禁，不等同于生产发布状态）
- `PLATFORM_EVOLUTION_PENDING=false`；`EVOLUTION_THREAD=NOT_CREATED`

实施完成、功能验证通过；自动视觉验收仍为 partial。真实手机安装与生产发布身份须按发布回执单独记录。

## 生产交付

- 后端/PWA：`v0.3.1810`，来源 `994e32ff36c6059e1e6c819bce5c958fc107904f`；健康检查和版本回读通过。未认证 roster 请求返回 401。
- Win/PC：最终前端 `4fecbc2b58c1ea41a6ddcf33184cd69f6695acd5`，`frontend_only`，兼容上述后端；`/pc` 与 release marker 通过。Win 默认打开线上 `/pc`，无需为本次业务页面另发节点程序。
- APK：`1.1.1846` / build `1846`，来源 `4fecbc2b58c1ea41a6ddcf33184cd69f6695acd5`，发布 SHA-256 `43fe72c7aab5265179515d069682fea63b339108ed3ef0118b3a62c5215bbed0`；manifest、177 项源码资源和服务器回读通过。
- 最后补充 PC 分页期间失去权限的清理。新增测试修复前失败，修复后浏览器、build、lint 全部通过；证据 `group-members-pagination-revoked-fixed-20261001-201000-550`。
- APK postflight 已查询主项目登记清单。HONOR AAK-AN00 与小米 23116PN5BC 均 offline，`REAL_DEVICE_STATUS=OFFLINE`、`ANDROID_RENDERER=emulator-5554`；本次没有自动安装成功的真机，不将模拟器验收描述为真机验收。
- 安装包：`http://43.139.149.158:8080/app/ElonAI-latest.apk`。用户可更新 APK；Win/PWA 重新加载群聊后使用新界面。
- 功能实现、功能回归、生产发布已完成；完整跨端自动视觉验收仍为 partial。未执行真实成员移除、解散或邀请作为生产验收。

发布回执：`group-members-publish-server-retry-20261001-195355-620`、`group-members-publish-pc-final-20261001-201446-923`、`group-members-publish-apk-20261001-201446-927`。首次服务端构建因共享 UNC build-dir 导致 Windows linker LNK1561 失败，重试仅把进程 `CARGO_BUILD_BUILD_DIR` 对齐既有官方 musl target；未修改全局缓存设置。

此文与注册表为发布后的文档登记，不改变已发布程序输入。最终工作区收尾以 `finish-ai-task.ps1` 回执为准。