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

## Sui 离线交接包预检

SDK 可在不安装 Sui SDK、不读取钱包和不连接 RPC 的情况下，检查平台导出的 `task_economy.sui_adapter_handoff.v1` 文件。校验内容包括固定字段、目标网络、标准或纠正包原子性、未提交状态、零提交次数、离线约束和 `handoff_digest`。

```js
import {
  createSuiPreflightClient,
  verifySuiAdapterHandoff,
} from '@elon/open-commerce-connector'

const verified = verifySuiAdapterHandoff(handoffJson)

const client = createSuiPreflightClient({
  baseUrl: 'https://commerce.example.com',
  token: process.env.ELON_SUI_PREFLIGHT_TOKEN,
})

await client.report(handoffJson, {
  outcome: 'passed',
  summary: 'deterministic offline preflight passed',
  toolVersion: 'merchant-adapter/1.0',
  idempotencyKey: `preflight-${verified.projectionPackageId}`,
})
```

命令行默认只做本地检查；只有显式增加 `--report` 才向平台提交报告。机器 Token 只能通过环境变量传入，不支持把 Token 放入命令参数。

```powershell
node bin/sui-preflight.mjs --handoff .\sui-handoff.json

$env:ELON_SUI_PREFLIGHT_TOKEN = 'sui_preflight_...'
node bin/sui-preflight.mjs --handoff .\sui-handoff.json --report `
  --base-url https://commerce.example.com `
  --idempotency-key preflight-job-001
```

该工具只证明本地文件符合当前离线契约并可向平台追加预检意见。它不构建 PTB、不签名、不广播、不确认最终性，也不移动资金。

项目编辑者把投影包显式加入预检队列后，机器可用独立任务客户端领取短时租约。领取结果会在内存中复核任务与交接包的项目、类型、网络和摘要绑定；SDK 不持久化适配器令牌或租约令牌。

```js
import { createSuiPreflightJobClient } from '@elon/open-commerce-connector'

const jobs = createSuiPreflightJobClient({
  baseUrl: 'https://commerce.example.com',
  token: process.env.ELON_SUI_PREFLIGHT_TOKEN,
})

const poll = await jobs.claimNext({ leaseSeconds: 300 })
if (poll.claimed) {
  const { job, handoff, lease_token: leaseToken } = poll.issue
  const outcome = await runDeterministicOfflineChecks(handoff)
  await jobs.complete(job.id, leaseToken, {
    outcome: outcome.passed ? 'passed' : 'rejected',
    summary: outcome.summary,
    toolVersion: 'merchant-preflight/1.0',
    idempotencyKey: `preflight-${job.id}-${job.attempt_no}`,
  })
}
```

命令行支持 `claim/renew/release/complete`。适配器令牌和后续命令使用的租约令牌都只从环境变量读取；`claim` 只把不含租约令牌的交接包写入新文件，并在标准输出中一次性返回租约令牌，调用方应立即放入受控环境变量且不得记录日志。领取不会自动完成任务。

```powershell
$env:ELON_SUI_PREFLIGHT_TOKEN = 'sui_preflight_...'
node bin/sui-preflight.mjs claim --base-url https://commerce.example.com `
  --output .\sui-preflight-handoff.json

$env:ELON_SUI_PREFLIGHT_LEASE_TOKEN = 'sui_preflight_lease_...'
node bin/sui-preflight.mjs complete --base-url https://commerce.example.com `
  --job-id sui_preflight_job_... --idempotency-key preflight-attempt-001
```

`renew` 和 `release` 同样要求 `--job-id` 与租约环境变量。所有任务命令都强制非本机地址使用 HTTPS、限制响应体大小，并且不包含 Sui SDK、钱包、RPC、签名或广播逻辑。

持续运行的离线预检进程可使用 `createSuiPreflightWorker`。工作器一次只处理一条任务，在租约临近到期时续租；正常结果原子完成，普通异常或进程中止会尽力释放任务，网络不可达时则依靠租约到期回收。幂等键由任务 ID 和尝试次数稳定生成。

```js
import { createSuiPreflightWorker } from '@elon/open-commerce-connector'

const worker = createSuiPreflightWorker({
  baseUrl: 'https://commerce.example.com',
  token: process.env.ELON_SUI_PREFLIGHT_TOKEN,
  toolVersion: 'merchant-preflight/1.0',
  async handler(handoff, { signal }) {
    const result = await runDeterministicOfflineChecks(handoff, { signal })
    return {
      outcome: result.passed ? 'passed' : 'rejected',
      summary: result.summary,
    }
  },
})

const shutdown = new AbortController()
process.once('SIGTERM', () => shutdown.abort('SIGTERM'))
process.once('SIGINT', () => shutdown.abort('SIGINT'))
await worker.run({ signal: shutdown.signal })
```

工作器的 `handler` 只能返回 `passed/rejected` 和说明，不会收到适配器令牌或租约令牌，也不能借此获得签名、广播或资金权限。

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

## 商户运行时内核

`createMerchantRuntime` 抽取了咖啡店参考节点中所有商户共同需要的协议层：HMAC 签名、5 分钟重放窗口、商户身份、Grant 检查、订单明确确认、幂等占位与重放、能力分发、Manifest 摘要和标准结果/错误信封。商户只实现自己的商品、库存、报价和订单处理器。

## 消费者可携带数据包签名

`signConsumerPortabilityPackage` 使用运营方 RSA 私钥签署固定协议消息，绑定来源运营方、公钥摘要、导出包标识、来源项目、幂等键、负载 SHA-256 和创建时间。接收方可用 `consumerPortabilityPublicKeyId` 登记公钥，并用 `verifyConsumerPortabilityPackageSignature` 在上传前复核。私钥只由来源运营方持有，不上传平台；签名证明某个已信任密钥签过该包，不会自动恢复关系、Grant、ERP、订单或资金状态。

`encryptConsumerPortabilityArchive` 和 `decryptConsumerPortabilityArchive` 提供可互操作的 PBKDF2-SHA256（310000 次）与 AES-256-GCM 离线归档。口令只存在于调用方进程，归档包含随机盐、随机 Nonce、认证标签和明文 SHA-256，不负责口令找回或云端密钥托管。

```js
import {
  createMemoryMerchantRuntimeIdempotencyStore,
  createMerchantRuntime,
} from '@elon/open-commerce-connector'

const runtime = createMerchantRuntime({
  merchantId: process.env.YILONG_MERCHANT_ID,
  keyId: process.env.YILONG_RUNTIME_KEY_ID,
  secret: process.env.YILONG_RUNTIME_SECRET,
  // 仅用于本机开发；生产环境必须替换为数据库实现。
  idempotencyStore: createMemoryMerchantRuntimeIdempotencyStore(),
  capabilities: [{
    key: 'catalog.search',
    access: 'public',
    input_schema: { type: 'object' },
  }],
  handlers: {
    async 'catalog.search'(input, context) {
      return merchantRepository.searchProducts(input, context)
    },
  },
})

// HTTP 框架必须把未经重新序列化的原始请求体和请求头交给内核。
const response = await runtime.handleInvoke({ headers, body: rawBody })
```

内存幂等存储只供本机开发和原型使用，进程重启会丢失记录。生产商户必须实现 `claim/complete/release` 持久化接口，并在数据库中对“商户 + App + 能力 + 幂等键”建立唯一约束；商品扣减、报价消费和订单创建仍由商户数据库事务负责。

## 数据边界

- 单页最多 500 条标准化变更。
- `runKey` 同时是同步回执的幂等键；相同键必须产生相同回执。
- 控制面只接收记录数量、状态、时间、错误代码和游标摘要，不接收原始订单或客户值。
- Manifest、健康证据和同步回执禁止携带 Token、Cookie、密码、API Key 或其他凭据字段。
- 原始经营数据应写入商户选择的数据存储，再通过 Capability 暴露最小授权能力。
- 任务客户端一次只读取一个与机器身份绑定的结果；完成或释放都需要同一个一次性租约密钥。

## 与服务端的关系

服务端 `GET /api/open-commerce/connector-contract` 返回当前契约版本和限制。兼容性测试通过只说明连接器符合技术契约，不代表已经获得某家平台的官方授权，也不代表生产稳定性已通过业务验收。
