# 一龙开放商业连接器 SDK

该 SDK 用于把 POS、商户导出文件、本地自动化或官方 API 适配器接入一龙开放商业控制面。它只定义厂商无关的连接器边界，不包含美团、抖音、京东或淘宝闪购的生产凭据和实现。

## 安装和验证

当前包随仓库发布，可在目录内直接运行兼容性测试：

```powershell
npm test
```

连接器必须实现三个方法：

- `describe()`：声明连接器、接入方式、授权范围和数据域。
- `health(context)`：返回带时间和证据代码的健康状态。
- `sync(request)`：按有界分页返回标准化变更。

```js
import {
  CONNECTOR_CONTRACT_VERSION,
  CONNECTOR_SCHEMA,
  defineConnector,
  runConnectorCompatibility,
} from '@elon/open-commerce-connector'

const connector = defineConnector({
  describe() {
    return {
      schema: CONNECTOR_SCHEMA,
      contractVersion: CONNECTOR_CONTRACT_VERSION,
      connectorKey: 'my-pos',
      providerKey: 'my-company',
      displayName: 'My POS',
      connectionMode: 'official_api',
      scopes: ['read.orders'],
      dataDomains: ['orders'],
    }
  },
  async health() {
    return {
      status: 'ready',
      observedAt: new Date().toISOString(),
      evidenceCode: 'official_api_authenticated',
    }
  },
  async sync(request) {
    return {
      receiptKey: request.runKey,
      syncKind: request.syncKind,
      status: 'succeeded',
      changes: [],
      startedAt: new Date().toISOString(),
      completedAt: new Date().toISOString(),
    }
  },
})

await runConnectorCompatibility(connector, {
  request: {
    integrationId: 'integration-id',
    runKey: 'unique-run-key',
    syncKind: 'incremental',
    dataDomains: ['orders'],
    limit: 100,
  },
})
```

## 数据边界

- 单页最多 500 条标准化变更。
- `runKey` 同时是同步回执的幂等键；相同键必须产生相同回执。
- 控制面只接收记录数量、状态、时间、错误代码和游标摘要，不接收原始订单或客户值。
- Manifest、健康证据和同步回执禁止携带 Token、Cookie、密码、API Key 或其他凭据字段。
- 原始经营数据应写入商户选择的数据存储，再通过 Capability 暴露最小授权能力。

## 与服务端的关系

服务端 `GET /api/open-commerce/connector-contract` 返回当前契约版本和限制。兼容性测试通过只说明连接器符合技术契约，不代表已经获得某家平台的官方授权，也不代表生产稳定性已通过业务验收。
