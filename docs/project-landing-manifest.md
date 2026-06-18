# 项目首页 Manifest 契约

PC 网页端打开项目时，默认先展示主项目渲染的“项目介绍与下载”首页。子项目可以在项目根目录提供 `.elon/project-landing.json`，让主项目读取简介、下载入口、适用人群和相关链接。

## 设计边界

- 主项目负责统一 UI、字段净化、平台下载按钮和默认兜底文案。
- 子项目负责提供真实内容和下载元数据，不嵌入整个子项目页面。
- `custom_landing_url` 只作为“完整介绍/官网”入口，不替代主项目首屏。
- `landing_manifest_url` 可以指向远端 manifest，当前阶段仅展示为资源链接，不在请求路径内联网抓取。
- PC 节点上的本地路径如果云端服务器不可访问，云端打开项目时暂时读不到本地 manifest；本机 node-agent 的目录检查接口已经返回同结构，后续可接注册同步。

## 文件位置

推荐：

```text
.elon/project-landing.json
```

兼容：

```text
.elon/landing.json
```

Manifest 最大 256 KB。字段只接受 JSON，不执行 HTML 或脚本。URL 目前只接受 `http://`、`https://` 和站内 `/path`。

## 示例

```json
{
  "schema_version": 1,
  "title": "一龙网游加速器",
  "tagline": "面向游戏玩家的多端加速客户端",
  "summary": "提供 Android 和 Windows 客户端下载，帮助用户快速安装并加入项目协作。",
  "highlights": [
    "Android / Windows 下载集中展示",
    "保留公告、文档、问题反馈和 AI 开发频道",
    "可扩展到 iOS、macOS、Linux 和网页端"
  ],
  "target_users": [
    "首次从 APK 进入项目的用户",
    "需要在 PC 端下载客户端的成员"
  ],
  "downloads": {
    "android": {
      "label": "Android APK",
      "manifest_url": "https://example.com/app/version.json",
      "url": "https://example.com/app/latest.apk",
      "version": "1.0.0",
      "size_label": "45 MB",
      "status": "available",
      "note": "手机端安装包"
    },
    "windows": {
      "label": "Windows 客户端",
      "manifest_url": "https://example.com/app/windows-version.json",
      "url": "https://example.com/app/windows/latest.exe",
      "version": "1.0.0",
      "size_label": "80 MB",
      "status": "available"
    },
    "ios": {
      "label": "iOS 教程",
      "status": "unavailable",
      "note": "教程页未发布时不要配置可点击下载 URL"
    },
    "macos": {
      "status": "planned",
      "note": "等待项目配置"
    },
    "linux": {
      "status": "planned"
    },
    "web": {
      "label": "网页端",
      "url": "https://example.com",
      "status": "external"
    }
  },
  "system_requirements": [
    "Android 8.0+",
    "Windows 10/11 64 位"
  ],
  "recent_updates": [
    "新增 Windows 客户端下载入口",
    "修正 Android 版本信息"
  ],
  "privacy_notes": [
    "客户端需要网络访问权限",
    "不在首页展示敏感密钥或本机绝对路径"
  ],
  "resources": [
    {
      "label": "完整介绍",
      "url": "https://example.com/promote"
    }
  ],
  "custom_landing_url": "https://example.com/promote",
  "landing_manifest_url": "https://example.com/project-landing.json"
}
```

## 平台与状态

支持的平台键：

- `android`
- `windows`
- `web`
- `ios`
- `macos`
- `linux`

支持的状态：

- `available`：有可用下载或访问 URL。
- `external`：外部入口，例如官网、教程、第三方客户端。
- `unavailable`：暂不可用，应该展示原因，不应配置可点击下载 URL。
- `coming_soon`：即将支持。
- `pending`：有 manifest 或待检查数据，但还没有稳定下载 URL。
- `planned`：未来计划支持。

## 子项目接入建议

子项目生成或发布客户端时，同步更新 `.elon/project-landing.json`。对于 bb64a 这类已有推广页的项目，推荐把现有 Windows Promote 页里的下载数据抽成 manifest，主项目首页展示“应用详情/下载”，Promote 页作为 `custom_landing_url` 的完整介绍入口。
