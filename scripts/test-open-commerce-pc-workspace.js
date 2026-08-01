const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const featureRoot = 'pc-frontend/src/features/open-commerce'
const panel = read(`${featureRoot}/OpenCommercePanel.tsx`)
const consumer = read(`${featureRoot}/ConsumerCommerceSandbox.tsx`)
const developer = read(`${featureRoot}/DeveloperCommercePortal.tsx`)
const clientApi = read(`${featureRoot}/openCommerceClientApi.ts`)
const resources = read(`${featureRoot}/AiResourceControlPanel.tsx`)
const economy = read(`${featureRoot}/ShadowEconomyPanel.tsx`)
const resourceService = read('server/src/ai_resource_control/service.rs')
const economyProjection = read('server/src/task_settlement/sui_projection.rs')
const runtimeManager = read(`${featureRoot}/OpenCommerceRuntimeManager.tsx`)
const runtimeClient = read('server/src/open_commerce_runtime_client.rs')
const directoryPublisher = read(`${featureRoot}/OpenCommerceDirectoryPublisher.tsx`)
const directoryContract = read('contracts/open-commerce/directory-merchant-v1.schema.json')

for (const view of [
  'OpenCommerceMerchantWorkspace',
  'ConsumerCommerceSandbox',
  'DeveloperCommercePortal',
  'AiResourceControlPanel',
  'ShadowEconomyPanel',
]) {
  assert.ok(panel.includes(view), `open commerce workspace must route ${view}`)
}

assert.ok(consumer.includes('ranking_is_paid'), 'consumer sandbox must expose paid-ranking state')
assert.ok(consumer.includes('requestAuthorization'), 'consumer sandbox must support explicit grants')
assert.ok(consumer.includes("appId === 'pc-web'"), 'public browser identity must stay explicit')
assert.ok(consumer.includes('app_registration_required'), 'restricted capabilities must require a registered app')
assert.ok(directoryPublisher.includes('setDirectoryPublication'), 'merchant UI must explicitly publish or withdraw directory access')
assert.ok(directoryPublisher.includes('项目 ID、所有者、运行地址、密钥和处理器配置不会公开'), 'merchant UI must explain the sanitized directory boundary')
assert.doesNotMatch(directoryContract, /project_id|owner_user_id|handler_type|handler_config/, 'public directory contract must not expose internal ownership or handlers')

assert.ok(clientApi.includes("'x-elon-app-id'"), 'signed-in app calls must bind an app identity')
assert.ok(clientApi.includes('Authorization: `Bearer ${testToken}`'), 'developer calls must use test credentials')
assert.doesNotMatch(developer, /localStorage|sessionStorage/, 'developer credentials must not persist in browser storage')
assert.ok(developer.includes('token_visible_once') || developer.includes('一次性测试凭据'), 'credential UI must state one-time visibility')

assert.ok(resources.includes('不会执行任务'), 'resource preview must be visibly non-executing')
assert.ok(resourceService.includes('execution_started: false'), 'resource preview backend must not start execution')
assert.ok(resourceService.includes('quota_verified: false'), 'external quota must remain unverified')

assert.ok(economy.includes('SHADOW ONLY'), 'economy UI must label shadow mode')
assert.ok(economy.includes('network_submission'), 'economy UI must show submission state')
assert.ok(economyProjection.includes('not_submitted'), 'Sui projection must remain unsubmitted')

assert.ok(runtimeManager.includes('credential_ref'), 'runtime UI must submit a server-side credential reference')
assert.doesNotMatch(runtimeManager, /shared_secret|明文密钥/, 'runtime UI must never collect the shared secret')
assert.ok(runtimeManager.includes('verifyRuntime'), 'runtime UI must require an explicit signed verification')
assert.ok(runtimeClient.includes('x-yilong-runtime-signature'), 'runtime calls must carry an HMAC signature')
assert.ok(runtimeClient.includes('merchant_runtime.result.v1'), 'runtime results must use the versioned contract')

console.log('Open commerce PC workspace contracts passed')

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), 'utf8')
}
