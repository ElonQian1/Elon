# PWA 聊天阅读密度调整

日期：2026-09-30。交付证据，不定义新的产品规范。

## 原因与调整

用户截图是 PWA 当前问题：每条消息后的“···”单独占据至少 48 px，自己的群消息另有至少 48 px 的编辑行。空的修改操作容器也带来留白；累加后远超消息正文。

- 消息操作按钮放在气泡旁边，保留 48×48 px 热区、可读名称、键盘操作与原长按入口。
- 编辑、查看修改记录进入同一消息操作菜单，继续调用原编辑与版本冲突流程。“已编辑”状态移至昵称旁。
- 隐藏仅限已经挂载操作菜单的当前消息修改栏；原修改模块和权限判断不变。
- 消息间距由 10 收至 6 px，昵称与正文间距收紧；正文保持 16 px，气泡内边距和时间分隔留白适当减少。
- 群聊、好友聊天复用该布局；不修改 APK、后端 API、发送权限、原始消息或用户字号选择。

## 运行证据

同一组合成消息、相同 844 px 高视口，WebKit 浅深主题结果一致：

| 宽度 | 调整前完整可见消息 | 调整后完整可见消息 | 8 条消息总高度变化 |
|---|---:|---:|---:|
| 320 px | 3 | 7 | 1378 → 678 px |
| 390 px | 3 | 8 | 1330 → 649 px |
| 430 px | 4 | 8 | 1306 → 635 px |

消息区高度为 683 px。数量取决于内容长度、图片及系统窗口，不承诺所有真机固定显示 8 条。

- `test-pwa-chat-density.cjs`：WebKit/Chromium 各 11 场景，覆盖浅深色、320/390/430 px、150/200% 文字、可点击菜单、编辑保存、修改记录、聊天视口缩短后的引用/输入；无横向溢出。
- `test-pwa-message-actions.cjs`：两引擎回归长按、图片点击放大、二维码、引用、复制、转发、多选、失败恢复及焦点；只写本地合成数据。
- `test-pwa-voice-playback.cjs`：WebKit 回归语音播放、暂停、重播、下载、错误重试和离开聊天释放。
- V2 规范 12 项自测及检查通过；JS 语法、源码规模、文档和差异检查按提交门禁执行。
- 测试禁止访问 loopback 外地址，没有操作生产群聊。实际 iPhone 键盘、手势、性能及用户审美验收仍待复验。

## UI 工作台

任务 `desktop_bcf236471ff74ed3bad62770c459d8a9` 导入了用户现状截图，没有干净目标图。`ui_capture_pwa_runtime` 捕获生产页面代码和本地合成群聊，390×844；4 步交互、0 脚本异常，浏览器进程及临时目录清理成功。最终提交的源码绑定、PNG 哈希和测试收据另存本任务交付工件。

通用完成门禁仍要求 Android debug runtime，返回 `PREPARATION_REQUIRED`。本次仅 PWA 消息布局和操作入口调整，不启动 APK 发布，也不宣称跨端视觉验收通过。

`FIT_RUN_STATUS=NOT_RUN; FINAL_VISUAL_LOSS=NOT_MEASURED; VISUAL_ACCEPTANCE_THRESHOLD=NOT_SET; CROSS_PLATFORM_VISUAL_PARITY=VERIFICATION_DEFERRED; BUSINESS_DELIVERY_READY=false; PLATFORM_EVOLUTION_PENDING=false; EVOLUTION_THREAD=none; REAL_DEVICE_STATUS=NOT_VERIFIED; ANDROID_RENDERER=NOT_USED`。

## 发布与用户复验

修改限于已有页面内嵌的 JS/CSS，提交并推送后由 `publish-mobile-pwa-static.ps1` 原子发布完整页面。无需新增服务端路由或 APK 构建。线上模板摘要、嵌入资源内容、发布身份与统一收尾以任务收据为准。

重新打开 PWA，查看群聊和好友聊天的紧凑消息；点气泡旁“···”检查编辑和修改记录，并检查长消息、图片、语音、引用、转发和多选。实际手机显示密度以用户复验为准。
