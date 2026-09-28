---
name: apk-ui
description: "一龙 APK/PWA 的 V2 设计重建、渐进 Compose 迁移与可访问性验收；旧轨道金属外观不再优先。"
---

# 一龙移动设计 V2 执行入口

先读 AGENTS.md、AI_CURRENT.md、docs/design/mobile-design-system-v2.md、docs/design/mobile-design-sources.md 和 docs/Design.md。双端任务再读 .github/instructions/apk-web-ui-sync.instructions.md；业务与交付按命中合同读取，不全量加载所有 skill。

本 skill 只编排 V2，不创建第三套设计权威。旧颜色、截图、组件外观和旧包 apk_refactor_phase1.zip 不再是自动执行入口。仅复用符合 V2 的组件；保留能力不等于保留位置、布局或皮肤。

先盘点能力与迁移路径 → 清理冲突规范/检查 → 建 Material 3 主题组件 → 完成代表页面 → 取得运行证据 → 分批推广。新组件优先 Compose，原有 View/WebView 按互操作边界过渡；不为 skill 切到 Expo，不重做会话和服务端。

检查实际触控、读屏语义、对比度、字体放大、键盘、安全区、状态恢复与真实数据。测试 ID 和用户可读描述一起迁移。社区 mobile-design 仅作补充，不能覆盖官方依据；调用任何官方迁移或设备 skill 前先读当前入口和前置条件，不把可见当已安装或已运行。

规范检查：python scripts/check-mobile-design-v2.py --self-test，然后 python scripts/check-mobile-design-v2.py。可用 --peer-root 比对另一仓的同版正文。检查只证明已列出的治理合同，不证明全仓清理或界面合格。

系统级 UI 先内部构建、关键原生运行验证，再受控正式发布。签名、授权、工作区和并发保护不变；缺少环境明确记未执行。分别报告规范、代码、测试、构建、运行、性能、发布及用户验收。
