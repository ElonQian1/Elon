# APP 颜色规范

最后更新：2026-06-13

本规范用于一龙 APP 的暗色 UI 配色。任何 APP 页面、组件、按钮、卡片、底部导航、状态胶囊、链接文字或主题色调整，都必须优先遵守本文件；除非任务本身是更新本规范。

当前体系可以定义为 **Elon 黑灰主色体系**：接近 ChatGPT 手机版的暗色比例，黑/深灰承担 85%-90% 的视觉面积，银灰承担主按钮、输入区和层级强调，原有绿色只作为 2%-4% 的状态、链接和进度点缀。

## 颜色 Token

| 用途 | 建议 Token | HEX |
|---|---:|---:|
| 页面总背景 | `color.bg.app` | `#101010` |
| 主卡片背景 | `color.surface.card` | `#222222` |
| 次级深色区域 / 我的节点背景 | `color.surface.subtle` | `#151515` |
| 分割线 / 弱描边 | `color.border.subtle` | `#2E2E2E` |
| 主标题 / 主要数字 | `color.text.primary` | `#D6D6D6` |
| 正文说明文字 | `color.text.secondary` | `#A8A8A8` |
| 弱提示 / 空状态文字 | `color.text.tertiary` | `#777777` |
| 次按钮背景，比如“查看明细” | `color.button.secondary.bg` | `#2A2A2A` |
| 次按钮文字 | `color.button.secondary.text` | `#D6D6D6` |
| 聊天输入胶囊背景 | `color.input.pill.bg` | `#303030` |
| 项目空间公告 / 项目介绍背景 | `color.project_space.info.bg` | `#30333A` |
| 主按钮背景，比如“发送” | `color.button.primary.bg` | `#C8C8C8` |
| 主按钮文字 | `color.button.primary.text` | `#101010` |
| APP 点缀色 / 成功、在线、完成状态 | `color.accent.primary` | `#58BE6A` |
| 链接文字，比如“进入节点算力市场” | `color.link.primary` | `#58BE6A` |
| 点缀胶囊背景，比如“积分明细” | `color.badge.info.bg` | `#16251A` |
| 点缀胶囊文字 | `color.badge.info.text` | `#8DDC9B` |
| 状态胶囊背景，比如“未配置” | `color.badge.neutral.bg` | `#2A2A2A` |
| 状态胶囊文字 | `color.badge.neutral.text` | `#D0D0D0` |
| 底部导航背景 | `color.nav.bg` | `#222222` |
| 底部导航选中背景 | `color.nav.active.bg` | `#242424` |

## 使用原则

- 核心底色使用 `#101010` / `#222222`，不要随意引入新的黑灰色阶。
- 信息容器统一使用黑灰体系，优先复用 `#222222`、`#2A2A2A`、`#151515`。
- 主操作、发送按钮、用户气泡优先使用银灰 `#C8C8C8`，保持克制的 ChatGPT 风格。
- 绿色 `#58BE6A` 只作为点缀色，用于在线、完成、成功、进度、链接，以及“充值额度”这类需要强行动召唤的商业入口。
- 聊天底部输入胶囊固定使用 `#303030`，保持图形上明显区别于底栏、比按钮更克制。
- 项目空间的“公告”和“项目介绍”使用 `#30333A`，与项目空间悬浮按钮保持同色，承载社区信息块时需配浅色文字；Web 侧用 `--project-space-info-bg` 表达同一颜色。
- 不再使用蓝色作为主要辅助色；历史蓝色入口统一收敛为绿色点缀或中性灰。
- 新增 APP UI 配色时，优先映射到现有 token；确实需要新增颜色时，应先更新本规范。
