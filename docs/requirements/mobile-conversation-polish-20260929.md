---
version_status: current
decision_status: accepted
reviewed_at: 2026-09-29
---

# 主应用双端消息界面打磨

用户反馈 iPhone PWA 输入面板粗糙、与 APK 相差较大，主 APK 暗色消息页紫色过多且缺少层次。两张附件是现状，不是目标设计。

## 设计决定

保留主 APK 默认暗色和用户显式选择；PWA 继续支持系统、浅色、深色。主项目 token 2.2 使用中性表面与克制的蓝色操作，替换初始 Material 紫色。色值只在 mobile-tokens-v2.json 中维护并生成两端资源；不改量化仓库或状态/金融语义色。系统字体、正文 16、次文 13–14、元信息 12 为页面起点，保持系统缩放。

消息是主体：AI 摘要缩为两层信息，类别徽标和拼图头像退为中性色；筛选与导航保留选中状态。PWA 聚合群助手/文章辅助入口，保持总结和消息区的明确边界；不移除功能。输入面板按内容增长，单行时紧凑、长草稿内部滚动、48px 操作热区，焦点只强调组件外边界。可点击头像同时约束宽高，防止触控最小高度把照片拉成椭圆。

## 能力与编辑边界

保留筛选、摘要跳转、好友/群/项目打开、未读、在线和工作状态、搜索、附件、表情、语音、草稿和发送。不执行真实消息或项目任务，不改权限/会话/后端。当前附件任务正在编辑 web_page.html、social_chat_view.js、social_source_compose.js 和基础 PWA fixture；本任务只在主题模板、共享 token、原生消息组件及独立 fixture 内修改。

## 验收

- 两端浅深主题采用同源 token，查看真实渲染页面；功能语义证明不能替代视觉检查。
- PWA 在 Chromium/WebKit、320/390/411px 检查焦点、单行/多行、正方形头像、图标、附件与草稿、缩短视口后的输入和发送可达性。WebKit 自动化不等同于 iPhone 系统键盘实测。
- Android 用生产页头、列表、导航与输入组件的离线预览，检查浅深主题、普通/200% 字体及空态；组件测试覆盖 320/411dp、100/150/200% 字体、触控、跳转和语义颜色对比。
- 用户反馈触发一次有界真机复核；环境失败明确延期，隔离模拟器完成原生证据后再正式发布。正式 APK 安装与手机画面验收分开报告。
- 发布后核对 PWA 线上工件和 APK 来源、版本、安装回执。无干净目标图，不宣称像素复刻；性能、真实 iOS 键盘和用户审美认可单列。

依据：[Android 语义颜色与层级](https://developer.android.com/design/ui/mobile/guides/styles/color)、[Web 焦点可见性](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Selectors/:focus-visible)。这是主项目页面设计选择，不把社区 Skill 或 Material 默认皮肤当作最终品牌标准。
