---
version_status: current
reviewed_at: 2026-09-16
---

# 社交外链卡片修复交付记录

需求：[卡片点击与呈现修复](../requirements/wechat-article-card-repair.md)。本轮只修复已有外链卡片的交互和显示，复用此前六平台识别、原文阅读和官方播放器能力。

## 根因和修改

- APK 长按菜单递归给卡片子 View 设置 longClickable，子 View 消耗普通点击。原生 View 手势用例复现旧行为；新卡片标题、来源、封面、容器共享打开动作，多选仍可接管。
- 独立链接和可识别分享口令显示紧凑暗色卡片，去除普通消息套框及重复原文。带评论、多链接、附件的消息保留内容；数据库原文和编辑历史不变。
- 小红书页面的通用网站标题不再覆盖分享文本中的具体标题、作者；PWA 卡片文字不再受消息链接颜色/下划线规则影响；PC 自己发送消息的样式不再覆盖 hidden 属性。
- 微信公开请求返回环境验证页，不能得到本篇标题/封面。APK 阅读器仅在用户正常打开并加载文章后读取公开标题、公众号名和封面 URL；同文章、同账号、同服务器、本机 128 条/24 小时内存缓存，不上传网页正文、Cookie 或凭证。未成功加载时保留可点击来源卡。
- 原始群链接由授权的只读数据库查询获得，仅筛选“杀蟑螂”中相关 HTTPS 平台消息。命中公众号 09:26、小红书 15:41、抖音 15:42；分享访问参数仅留本机临时私有证据，未提交。

## 验证边界

| 能力 | 实现 | 验证 | 发布/验收 |
|---|---|---|---|
| 子控件点击、长按、多选、原链接打开 | implemented | Android 实际 View 手势与显式 Intent 测试 | 正式发布后记录版本；真机待复核 |
| 消息复用、附加评论与附件 | implemented | 实际 ChatAdapter 绑定回归 | 同上 |
| 分享标题、缓存隔离与验证页拒绝 | implemented | Kotlin/JS 定向回归 | 公开微信元数据获取仍受平台验证限制 |
| PWA 卡片显示和阅读窗口 | implemented | 实际消息渲染函数、固定数据、Edge 截图与点击 | 非登录生产会话，不能代替真机内容验收 |
| PC | implemented | TypeScript/Vite 正式构建、来源链接回归 | 发布后核对线上资源 |

浏览器固定数据分别覆盖可用封面与无元数据；封面样例不是远端获取证明。已点击标题、来源、封面，打开/关闭抖音阅读对话框，保留聊天草稿；没有发送任何测试消息。

## 真机准备与 UI 门禁

用户反馈刚交付结果不正确，按 UI Skill 触发一次真机准备。调用发生在候选源码提交前，平台返回 `DEBUG_CANDIDATE_DIRTY`，未安装、启动或操作手机；遵守单次预算和停止信号，不重复准备。这是本轮准备顺序造成的验收缺口，不能记为手机故障或视觉通过。

- `REAL_DEVICE_STATUS=REQUIRED_FOLLOWUP`；`ANDROID_RENDERER=UNAVAILABLE`。
- `FIT_RUN_STATUS=NOT_RUN`；`FINAL_VISUAL_LOSS=N/A`；`VISUAL_ACCEPTANCE_THRESHOLD=N/A`。
- `CROSS_PLATFORM_VISUAL_PARITY=VERIFICATION_DEFERRED`；`BUSINESS_DELIVERY_READY=false`。
- `PLATFORM_EVOLUTION_PENDING=false`；`EVOLUTION_THREAD=none`。
- `ui_check_workflow_completion`：`PREPARATION_REQUIRED / DEBUG_RUNTIME_NOT_CONNECTED`，没有已证明的平台能力缺口，不派发平台升级任务。

源码/构建/正式发布与设备视觉验收分报；后续真机复核应从已提交的正式版本开始，检查上述三条群消息，不清除应用账号或聊天数据。

## 发布与证据

定向测试、工件版本、线上来源校验和统一收尾结果在发布后补入。最终截图与回执复制到工作区外的 `social-card-repair` 证据目录，避免随任务工作树清理丢失。
