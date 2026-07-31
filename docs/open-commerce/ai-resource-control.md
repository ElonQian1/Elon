# 项目 AI 资源控制面

本文说明项目级 AI 资源盘点、策略和路由预演。架构约束见 `docs/decisions/ai-resource-control-v1.md`。

## 当前资源类型

| 类型 | 来源 | V1 解释 |
|---|---|---|
| `own_codex` | 当前用户自己的 Codex 保险箱槽位 | 只显示脱敏标签和模型线索 |
| `remote_node` | 当前用户拥有且在线的 PC 节点 | 可作为“仅本地执行”的候选 |
| `shared_codex` | 其他用户显式授权给当前用户的 Codex | 只显示有效授权，不泄露凭据 |
| `platform_model` | 项目可见的平台模型配置 | 未实时验证外部余额 |

控制面保存的是项目策略，不复制资源凭据。资源清单是当前可见事实的投影，`available`、`unverified`、成本未知和额度未验证必须分别表达。

## 策略

项目编辑者可以配置：

- 启用哪些资源类型。
- 资源类型优先级。
- `prefer_local`、`balanced` 或 `prefer_available` 倾向。
- 主候选不可用时是否允许回退。
- 已知单位成本上限。

策略校验要求启用集合与优先级集合一致且无重复。设置成本上限时，未知成本资源不会被假设为低成本。

## HTTP 接口

```http
GET   /api/projects/{project_id}/ai-resources/overview
PATCH /api/projects/{project_id}/ai-resources/policy
POST  /api/projects/{project_id}/ai-resources/preview
```

预演响应必须包含：

```json
{
  "execution_started": false,
  "quota_verified": false
}
```

它只返回选中候选、回退候选和可解释原因，不创建 Assignment、不调用模型或节点，也不扣减任何额度。

## 验收

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 `
  -TargetDir D:\rust\shared\target -- test `
  --manifest-path server\Cargo.toml `
  ai_resource_control::tests

Set-Location pc-frontend
npm run test:open-commerce
```

真实任务路由仍由现有聊天、AI CLI、节点和计量模块负责。将策略接入实际执行器属于后续任务，不能仅凭预演结果宣称已完成统一调度。
