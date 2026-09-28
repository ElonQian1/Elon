---
version_status: draft
reviewed_at: 2026-09-28
---

# SEMANTIC_PARITY 契约评审

本文件是实现前评审，不改变现行工具或完成门禁，不代表原业务视觉验收通过。

## 已核实的缺口

移动设计 V2 允许 Android/PWA 采用各自平台布局，同时保持产品语义和设计版本一致。
`node_agent_android_live/cross_platform_verification.rs` 当前只接受 `VISUAL_PARITY`
与 `NO_WEB_COUNTERPART`。两端确实有对应页面但布局不同的场景，不能如实落入后一模式；
前一模式的像素阈值也不能直接表示语义一致。

`task_completion.rs` 接受 schemaVersion 1/2，验证当前 task/source 与文件、模式字段。
writer 会复制图片并记录哈希，但现有 visualLoss 来源是调用参数；这不能直接复用为
新模式的可信语义结论。只加枚举、接受调用者 `semanticParity=true`，或令 visualLoss=0
都会绕过需要补齐的验证，因此不采用。

## 建议的 schemaVersion 3

新模式应由正式 writer 验证并生成，示意输入如下；字段尚未进入工具 schema：

```json
{
  "taskId": "<当前设计任务>",
  "mode": "SEMANTIC_PARITY",
  "designSystem": { "id": "mobile-design-v2", "revision": "<可追溯版本>" },
  "androidProofId": "<本轮无补丁构建和连接 Runtime 的证明>",
  "webCaptureId": "<本轮真实浏览器 capture manifest>",
  "screenId": "login",
  "stateId": "signed-out",
  "fixtureRevision": "<双端相同测试状态版本>",
  "requirements": [
    {
      "id": "login-submit",
      "androidNode": "<原生语义节点标识>",
      "webNode": "<Web 语义节点标识>",
      "role": "button",
      "accessibleName": "登录",
      "enabled": true
    }
  ],
  "platformDifferences": [
    { "requirementId": "login-submit", "kind": "LAYOUT", "reason": "平台原生布局适配" }
  ]
}
```

提交方只提供映射意图和允许差异；是否通过必须由 writer 从可信证据计算。
设计版本、页面/状态及必验 requirements 应来自当前任务的已确认验收规格，不能由调用者
临时缩小到一个容易通过的节点。缺节点、重复/歧义映射或空集合均失败。

## 必须保留的证明链

1. 当前任务、规范项目根、当前源码 fingerprint 和设计系统版本相符；跨仓库对应方需
   显式登记，并分别验证源码 revision。不能仅比较两个调用者填入的字符串。
2. Android 使用本轮真实连接 Runtime：package/device、构建 revision、generation 与
   installed generation 一致；无 live patch/redo 历史；确定性源码写回和无补丁构建通过。
   不接受控制会话、bootstrap Runtime 或只存在于磁盘的旧截图作为替代。
3. Web 读取真实捕获 manifest、图片和语义树，重新计算哈希；验证 URL、viewport、fixture、
   source/route revision、认证模式和 `expectedPage`。公开登录页仅能对应 signed-out 状态。
4. 双端截图、语义树和行为回执绑定同一轮验收。路径须在受控证据目录内，拒绝替换、逃逸、
   过期源码和重复使用不匹配的 runtime/capture 身份。
5. 逐项检查角色、可访问名称、可用/选中/输入状态及任务规定的导航结果。涉及行为时必须有
   双端真实操作回执；仅凭文本相同不能宣称交互等价。
6. 平台差异只允许任务政策明确准许的布局/组件实现差异，不允许删除能力、状态或降低可访问性。
   语义模式仍保留双端截图供视觉评审，不能宣称像素损失为零。

## 门禁与兼容

- 保留现有两种模式及已有证据读取；新模式单独返回 `CROSS_PLATFORM_SEMANTIC_PARITY`。
- 分开报告原生来源验证、Web 来源验证、语义映射、行为断言和平台差异审查结果；任一必选项
  缺失时完成状态为 blocked，不用统一布尔值掩盖缺口。
- 有干净目标图的任务继续要求原有 accepted FitRun/source-parity 门禁；语义模式不能取消它。
- `strict_gate_passed` 和聚合完成逻辑只接受经过验证的新状态；不能单靠拓宽状态白名单完成接入。
- profile/task 必须明确允许 V2 语义模式。历史像素精确任务继续执行 `VISUAL_PARITY`。

## 实施与回归边界

新增独立 semantic verifier 和证据解析模块，现有 writer/完成入口只做分派，避免继续扩大
589/596 行左右的入口文件。可信 Runtime 校验需要异步 broker 查询，应复用现有 source proof
入口。工具 schema、manifest、completion gate 及错误码作为同一批次交付。

正例覆盖双端同状态、布局差异被明确允许；反例覆盖不同状态、设计版本漂移、错误源码、
旧 generation、活跃 patch、伪 Runtime、被篡改图片/语义树、空映射、缺/重复节点、认证模式不符、
行为未执行、目标图任务缺 FitRun、跨项目证据以及调用者伪造通过字段。

当前交付仅完成这份评审；新模式未实现、未发布。前台原生验收占用期间只进行源码与离线测试，
下一轮需在完整实现后单独安排节点升级和真实双端验收。
