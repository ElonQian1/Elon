# APP 颜色规范

最后更新：2026-06-01

本规范用于一龙 APP 的暗色 UI 配色。任何 APP 页面、组件、按钮、卡片、底部导航、状态胶囊、链接文字或主题色调整，都必须优先遵守本文件；除非任务本身是更新本规范。

当前体系可以定义为 **Elon 暗色冷灰体系**：深色底 + 冷蓝灰卡片 + 绿色主操作 + 蓝色辅助入口。

## 颜色 Token

| 用途 | 建议 Token | HEX |
|---|---:|---:|
| 页面总背景 | `color.bg.app` | `#101010` |
| 主卡片背景 | `color.surface.card` | `#181B20` |
| 次级深色区域 / 我的节点背景 | `color.surface.subtle` | `#0F1217` |
| 分割线 / 弱描边 | `color.border.subtle` | `#1E2126` |
| 主标题 / 主要数字 | `color.text.primary` | `#F2F5FA` |
| 正文说明文字 | `color.text.secondary` | `#A6AFBD` |
| 弱提示 / 空状态文字 | `color.text.tertiary` | `#6F7785` |
| 次按钮背景，比如“查看明细” | `color.button.secondary.bg` | `#283140` |
| 次按钮文字 | `color.button.secondary.text` | `#DDE8FC` |
| 主按钮背景，比如“充值额度” | `color.button.primary.bg` | `#58BE6A` |
| 主按钮文字 | `color.button.primary.text` | `#07120A` |
| 链接文字，比如“进入节点算力市场” | `color.link.primary` | `#6091CF` |
| 蓝色胶囊背景，比如“积分明细” | `color.badge.info.bg` | `#152C3E` |
| 蓝色胶囊文字 | `color.badge.info.text` | `#81B3D9` |
| 状态胶囊背景，比如“未配置” | `color.badge.neutral.bg` | `#283345` |
| 状态胶囊文字 | `color.badge.neutral.text` | `#B8C4D8` |
| 底部导航背景 | `color.nav.bg` | `#1E1E1E` |
| 底部导航选中背景 | `color.nav.active.bg` | `#262626` |

## 使用原则

- 核心底色使用 `#101010` / `#181B20`，不要随意引入新的黑灰色阶。
- 信息容器统一使用冷蓝灰体系，优先复用 `#181B20`、`#283140`、`#283345`。
- 只有真正的主操作使用绿色 `#58BE6A`。
- 入口、详情、跳转类操作使用蓝色 `#6091CF` 或深蓝胶囊 `#152C3E`。
- 新增 APP UI 配色时，优先映射到现有 token；确实需要新增颜色时，应先更新本规范。
