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

## 处理 ERP/CRM 衔接任务

获得项目编辑者显式签发且包含 `business_handoff.claim` 的限时机器 Token 后，适配器可复用 SDK 客户端领取一条任务。客户端不会要求或发送项目、商户、接入器 ID，这些边界由服务端从 Token 派生。

```js
import { createAdapterHandoffClient } from '@elon/open-commerce-connector'

const handoff = createAdapterHandoffClient({
  baseUrl: 'https://commerce.example.com',
  token: process.env.ELON_ADAPTER_TOKEN,
})

const poll = await handoff.claimNext({ leaseSeconds: 300 })
if (poll.claimed) {
  try {
    // 长任务可按需续租，但服务端不会允许超过首次领取后 1 小时。
    await handoff.renew(poll.issue, { extendSeconds: 600 })
    const targetReference = await writeToMerchantErp(poll.issue.task)
    await handoff.complete(poll.issue, {
      receiptKey: `erp-${poll.issue.claim.invocation_id}`,
      status: 'applied',
      targetDomain: 'erp',
      targetReference,
      completedAt: new Date().toISOString(),
    })
  } catch (error) {
    await handoff.release(poll.issue, 'transient_failure')
    throw error
  }
}
```

租约密钥只存在于本次 `poll.issue` 中；SDK 不写浏览器存储、文件或日志。非本机地址强制 HTTPS，响应体上限为 256 KiB。续租单次限制 60–900 秒，总处理期限不超过首次领取后 1 小时。主动释放只表示当前适配器放弃本次尝试，任务可被重新领取，不表示外部 ERP 已处理。提交 `rejected` 后，服务端按 30–900 秒有界退避并优先分配未尝试或最久未尝试任务，调用方无需自行制造高频重试；第 6 次拒绝后自动领取暂停，需由项目编辑者先排查外部故障并明确恢复。

生产接入器可使用通用工作器，把租约协议与商户业务处理函数分开。工作器顺序处理任务，自动续租、生成稳定回执键、重试幂等完成请求，并在临时故障或停机时释放任务。商户处理器必须使用 `idempotencyKey` 写入 ERP，避免“外部写入成功但回执网络中断”后重复创建订单。

```js
import {
  AdapterHandoffRejectError,
  createAdapterHandoffWorker,
} from '@elon/open-commerce-connector'

const worker = createAdapterHandoffWorker({
  baseUrl: 'https://commerce.example.com',
  token: process.env.ELON_ADAPTER_TOKEN,
  targetDomain: 'merchant_erp',
  async handler(task, { idempotencyKey, signal }) {
    const result = await merchantErp.applyBusinessResult(task.result, {
      idempotencyKey,
      signal,
    })
    if (result.permanentlyRejected) {
      throw new AdapterHandoffRejectError('erp_validation_failed')
    }
    return {
      status: 'applied',
      targetReference: result.referenceId,
    }
  },
})

const shutdown = new AbortController()
process.once('SIGTERM', () => shutdown.abort('SIGTERM'))
process.once('SIGINT', () => shutdown.abort('SIGINT'))
await worker.run({ signal: shutdown.signal })
```

普通异常默认按 `transient_failure` 主动释放；明确的永久业务拒绝使用 `AdapterHandoffRejectError`，容量不足或人工暂停可抛出 `AdapterHandoffReleaseError`。工作器不会保存机器 Token、租约密钥或原始任务，也不会替代商户 ERP 自身的幂等、事务和权限校验。

## 数据边界

- 单页最多 500 条标准化变更。
- `runKey` 同时是同步回执的幂等键；相同键必须产生相同回执。
- 控制面只接收记录数量、状态、时间、错误代码和游标摘要，不接收原始订单或客户值。
- Manifest、健康证据和同步回执禁止携带 Token、Cookie、密码、API Key 或其他凭据字段。
- 原始经营数据应写入商户选择的数据存储，再通过 Capability 暴露最小授权能力。
- 任务客户端一次只读取一个与机器身份绑定的结果；完成或释放都需要同一个一次性租约密钥。

## 与服务端的关系

服务端 `GET /api/open-commerce/connector-contract` 返回当前契约版本和限制。兼容性测试通过只说明连接器符合技术契约，不代表已经获得某家平台的官方授权，也不代表生产稳定性已通过业务验收。
