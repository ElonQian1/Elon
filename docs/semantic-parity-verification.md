---
version_status: current
reviewed_at: 2026-09-28
---

# V2 跨端语义状态验收

本能力为 `ui_write_cross_platform_verification` 的 `SEMANTIC_PARITY` 模式。
它验证明确声明的双端能力/状态映射，不产生像素相似度，不证明未执行的业务操作。
有干净目标图的任务仍须满足 FitRun/source-parity 门禁。

## 源码中的规格

在业务仓库提交 `docs/ui/semantic-parity/<taskId>.json`，包含本轮全部必验状态。
工具要求文件内容与 HEAD 一致，并校验设计系统和需求文件的 SHA-256；不允许调用时
临时减少状态或映射。文件可另选相对路径，但必须是本项目已提交文件。

以下是 schemaVersion 1 的形状，尖括号须用真实节点和值替换；单状态示例不是完整业务规格：

```json
{
  "schemaVersion": 1,
  "taskId": "mobile-v2-native-20260928",
  "scope": "OBSERVED_CAPABILITY_STATE",
  "designSystem": {
    "id": "mobile-design-v2",
    "sourceFile": "docs/design/mobile-design-system-v2.md",
    "sha256": "<该文件 SHA-256>"
  },
  "requirementSpec": {
    "sourceFile": "docs/requirements/mobile-design-v2-adoption.md",
    "sha256": "<该文件 SHA-256>"
  },
  "platformDifferences": ["Android 原生布局与 PWA 布局按 V2 适配；能力和状态保持对应"],
  "states": [{
    "id": "login-light-100",
    "androidScreenId": "elon.mobile.design_v2",
    "androidFontScale": 1.0,
    "webCapture": {
      "url": "http://127.0.0.1:55363/",
      "viewport": {"width": 411, "height": 842},
      "waitFor": {"selector": "#loginView"},
      "fixtureProfile": "mobile-v2-light",
      "expectedPage": {"kind":"PUBLIC_LOGIN","pageId":"main-login","path":"/","readySelector":"#loginView"}
    },
    "capabilities": [{
      "id": "login-submit",
      "state": "未登录，登录按钮显示且可用",
      "android": {
        "resourceId": "<真实 resourceId，含 namespace 时保留 namespace>",
        "assertions": {"/kind":"<真实 kind>","/text":"登录","/properties/<实际属性路径>":true}
      },
      "web": {
        "selector": "#loginBtn",
        "assertions": {"/role":"button","/label":"登录","/disabled":false}
      }
    }]
  }]
}
```

Android 选择条件支持 `definitionId`、`resourceId`、可选 `instanceKey`，至少提供前两项之一；
若同时提供则同时匹配。必须恰好命中一个节点，匹配当前 screenId、可见性和字体比例。
不要按存在的 selector 猜测状态；每项映射必须列出实际可观测断言。

每端 assertions 至少两项。Android 必须含 `/kind` 或 `/className`，另含 `/text`、
`/properties/...` 或 `/capabilities/...` 的真实状态。Web 必须含 `/role`，另可使用
`/label`、`/disabled`、`/checked`、`/selected`、`/inputType`、`/interactive`、
`/style/color`、`/style/backgroundColor`、`/style/fontSize`。值只允许非空字符串、数字、布尔值；
缺失属性不能用 null 冒充通过。原生未上报的状态会明确失败，需要补实际观察手段。

`webCapture` 使用现有 PWA 捕获 schema，省略 `evidence`（工具绑定实际工作区 revision）。
步骤仅允许等待、文字断言、滚动。认证资料仍使用正式 authProfile；公开登录页可声明
PUBLIC_LOGIN。不得写入账号、密码或 Cookie，也不得伪造登录后的 DOM。

## 分状态执行与整体门禁

将真实 Android Runtime 导航到该状态后调用：

```json
{
  "taskId": "mobile-v2-native-20260928",
  "verificationMode": "SEMANTIC_PARITY",
  "semanticContractPath": "docs/ui/semantic-parity/mobile-v2-native-20260928.json",
  "stateId": "login-light-100"
}
```

工具自行捕获当前应用进程帧/原生树和真实浏览器工件，检查当前源码、构建、代次及无 Patch
证明，然后生成节点认证回执。逐状态累计，缺任意规定状态时返回 PARTIAL，完成门禁仍失败。
完整状态集合通过后才返回 `CROSS_PLATFORM_SEMANTIC_PARITY=PASSED`。
结果没有 visualLoss；截图可供人工视觉评审。

本轮业务规格必须覆盖已确认的完整清单。无账号授权或登录后 fixture 时，保留对应状态，
明确缺证据；不可删状态、使用 NO_WEB_COUNTERPART 或以公开登录页替代。
此模式观察当前状态，不证明真实付款/发送消息/账号修改、系统 IME 或网络断开操作已经执行。
这些行为与设备事实仍需各自真实验收回执。

源码实现和部署状态独立；工具可用性以当前节点 tools/list 为准。
