# APP 低风险视觉修改快速通道

## V2 范围门禁

本入口服从 [移动设计系统 V2](design/mobile-design-system-v2.md) 和 `.github/copilot-instructions.md`。主题重建、公共组件替换、导航或信息架构变化、Compose 迁移、整页设计导入、触控/无障碍/键盘行为变化均不是低风险微调，必须退出本通道，先取得关键运行证据再正式发布。

只有既有合格组件内、影响可静态限定、不改业务/权限/交互/无障碍语义且不依赖硬件或 OEM 行为的小范围视觉参数变更可以进入。本通道不授权继续采用已退役颜色、旧皮肤或精确尺寸断言。

APK/PWA 按 [语义对齐规则](../.github/instructions/apk-web-ui-sync.instructions.md) 判断影响，不再强制同一提交修改 web_page.html；PWA 无影响时向验证器传 `-NoPwaImpactReason` 并写明理由，不制造空改动。

## 保留的受控执行入口

进入前完成原有任务预检、隔离和查重。使用 `scripts/invoke-ai-logged-command.ps1` 包装 `scripts/validate-app-ui-fast-lane.ps1`；有稳定行为合同则传 `-ContractTest`，否则用 `-NoContractReason` 说明，不复制颜色常量充当测试。

该验证入口负责其已支持范围内的 Android Debug 构建和 PWA 源码检查；不把它当作新 Compose 页面或未知资产的完整验收器。脚本若不能覆盖当前变更，退出通道，不改窄校验来伪造通过。

符合进入条件并完成验证后，沿原 Git 工作流提交，再使用 `scripts/publish-app-ui-fast-lane.ps1` 发布受影响端。正式签名、版本分配、构建覆盖检查、部署归属、任务基线、成功 push 路径集合和收据恢复继续由现有脚本负责，禁止另写发布捷径。

运行时 PWA 模板资产只在原脚本已支持且真实自包含时走模板发布；其他服务端或内嵌资产变化不能谎称纯模板。部署债务不自动变成本任务；未记录任务基线或远端已有更新时按脚本失败关闭。

保持原有顺序、互斥和幂等：受影响 Server/PWA → APK；不并行抢版本或用旧收据跳过远端 SHA/哈希回读。长命令保留超时、日志和子进程清理，Gradle 按项目脚本运行。

## 发布、装机与验收

只有上述低风险范围可使用 publish-before-optional-renderer。源码、测试、构建或发布失败仍阻断交付；无真帧不得称视觉通过。发布后登记手机的自动安装、保留数据、版本回读及失败报告继续按 `docs/apk-debug-device-delivery.md` 执行，不能因视觉检查可选而跳过。

可选 Renderer 准备前检查资源与 lease。没有空闲槽直接记录延期，不反复启动；存在空闲槽才允许原合同的一次有界准备。用户要求先验收、反馈修改不正确，或涉及系统级 V2 范围时，不再适用发布后补验的豁免。

收尾走原 `AndroidFeature` 合同，分别报告业务发布、视觉证据和装机结果。脚本入口与完整发布机制仍查 `.github/instructions/git-deploy-workflow.instructions.md`；发布脚本通过 `mobile-ui-design-scope.ps1` 拒绝已识别的主题源/主题资源重建；此守卫不替代对交互、导航和组件变化的人工语义判断。
