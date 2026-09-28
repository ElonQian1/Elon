---
version_status: current
decision_status: accepted
reviewed_at: 2026-09-28
implementation_status: in_progress
---

# 移动设计技能接入与取舍

当前用户要求：两个 APK 重新建立规范并系统重构，停止强制继承旧皮肤。本文是技能采用记录；设计权威仍是 mobile-design-system-v2.md。技能版本与入口摘要见 mobile-skills.lock.json。

## 已核查的实现环境

两个生产 APK 当前均为 Kotlin View/XML 与 WebView；主项目的 Compose 仅在调试渲染模块中。AGP 8.2.2、Kotlin 1.9.22、compile/target SDK 34、min SDK 26、JDK 17。主项目已依赖 Material Components 1.11.0；量化本批接入同版 Material Views。Material 3 可以先用于现有 View，不把整仓改 Compose 当成获得好设计的前提。

## 技能矩阵

| 技能 | 本机接入 | 本项目用途与边界 |
|---|---|---|
| mobile-android-design | 安装并读取入口、主题/布局参考 | 原生 Android、Material 3、状态提升、导航、无障碍；Compose 代码在有清楚互操作边界时采用 |
| mobile-design | 安装并读取入口及 Android、触控、设计思考、排版、测试参考 | 单手操作、可发现动作、字体缩放、离线/错误状态；不采用 RN/Flutter 专用实现 |
| ui-ux-pro-max | 安装，运行整体设计与 Compose 检索 | 作为建议库；整体检索两次均夹带营销落地页结构，未采用该结构。Compose 检索首次无匹配，缩小为 touch accessibility 后获得语义建议 |
| android-cli | 安装并读取入口 | Google 官方运行/截图/文档工具流程；本次设备操作仍走已有一龙工作台及租约，未将安装 skill 说成已安装 CLI 二进制 |
| edge-to-edge | 安装并读取前置条件、系统栏/IME 章节 | 学习单一 inset 消费与键盘可达性；现有生产 View/SDK 34 不满足该 Compose/SDK 35 迁移流程，不因此自动升级 SDK |
| expo-native-ui | 安装并读取入口 | 学习平台语义色、原生控件、可滚动内容；不引入 Expo、React Native 或 iOS 视觉规则 |
| expo-design-system | 安装并读取入口 | 采用单一 token 来源、组件变体与状态的思想；用户已明确撤销旧规范，不能机械执行“原系统永远优先” |
| frontend-design | 安装并读取入口 | PWA 的层级、真实文案、完整状态与视觉自检；营销 hero、特殊字体和装饰不作为原生应用要求 |
| Figma design-to-code | 已有插件，读取现行入口 | 有具体设计节点时取设计上下文与截图、适配原生布局；当前无用户 Figma 目标，不伪造匹配或验收证据 |
| Stitch / 一龙 UI 工作台 | 复用本地链路 | 设计导入、隔离模拟器与 PWA 运行证据；无目标图时不把任意截图宣称为像素还原 |
| apk-ui | 两仓项目入口 | 按任务调度适用规则，不全量加载、不创建第三套视觉权威 |

8 个外部 skill 安装在当前用户的 Codex skills 目录；后续新任务可重新发现。本任务直接读取文件应用。未把上游数百份无关技能拷进两个仓库，也未执行它们建议的框架迁移。

## 对技能本身的审查

- mobile-design 的表格把 WCAG 2.2 触控尺寸笼统写为 44px；WCAG 2.2 的 AA 2.5.8 为 24 CSS px（含例外），44px 属于增强级 2.5.5。Android 本项目采用官方建议的 48dp，不能混用 px/dp/pt。
- “所有可交互内容都写 contentDescription”需要细化：原生文字控件已有文本语义，额外重复描述会干扰读屏。图标按钮需要动作名称；机器 ID 放专用标记。
- 动态取色是可选能力，不覆盖金融涨跌与风险语义；“必须 SSL pinning”“永远 memoize”等社区口号不扩展本次 UI 改造范围。
- 以官方平台指导、真实源码和运行结果评审建议；“最新”“安装成功”“检索命中”都不等于设计正确。

## 本轮设计检查点

Android / Kotlin View，保留现有账号、会话、授权与交易边界。应用三条原则：同一语义色源管理浅深主题；布局按窗口与字体测量；外观、交互反馈和自动化定位分离。避免：按单张截图缩放全页；把业务页面模式当成整页主题；把网页模板当原生组件。

共享运行 token 在 mobile-tokens-v2.json，脚本生成 Android values/values-night；主项目同时生成 PWA 色层。旧资源名称仅为兼容别名。初始 Material 3 色板是待运行验收的基线，不把它宣布为最终品牌方案。

## 证据与后续验收

能力路径：主项目的项目分类、排序、打开与管理、AI 对话、附件和结果保持原业务回调；量化的网格/行情/资产/我的、账户切换、详情与离线草稿保持原模型及权限。旧机器 ID 同步迁移到专用标记，测试与检查接口一起更新。

必须分别记录：治理检查、源码编译、自动测试、浅深主题、320/411/600dp、长中文、200% 字体、键盘、离线/错误、运行截图、性能、发布和装机。缺少任何证据就记录未验证。系统级改造的原生运行证据先于正式发布。

官方依据：[Android 无障碍](https://developer.android.com/guide/topics/ui/accessibility/apps)、[Material Components 1.11.0](https://github.com/material-components/material-components-android/blob/1.11.0/docs/getting-started.md)、[WCAG 2.5.8](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html)。
