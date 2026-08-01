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
const developerPortal = read(`${featureRoot}/DeveloperCommercePortal.tsx`)
const outboundRequests = read(`${featureRoot}/OutboundAuthorizationRequests.tsx`)
const lifecycleService = read('server/src/open_commerce_client_lifecycle_service.rs')
const lifecycleApi = read('server/src/open_commerce_client_lifecycle_api.rs')
const clientApiRoute = read('server/src/open_commerce_client_api.rs')
const merchantWorkspace = read(`${featureRoot}/OpenCommerceMerchantWorkspace.tsx`)
const rateLimitManager = read(`${featureRoot}/OpenCommerceRateLimitManager.tsx`)
const rateLimitStore = read('server/src/store/open_commerce_rate_limits.rs')
const appBlockManager = read(`${featureRoot}/OpenCommerceAppBlockManager.tsx`)
const appBlockStore = read('server/src/store/open_commerce_app_blocks.rs')
const mcpTools = read('server/src/open_commerce_mcp_tools.rs')

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
assert.ok(developerPortal.includes('disableApp'), 'developer portal must allow an editor to disable a sandbox app')
assert.ok(developerPortal.includes('reactivateApp'), 'developer portal must issue a new one-time credential when reactivating')
assert.ok(outboundRequests.includes('撤回申请'), 'developer portal must show and cancel outbound authorization requests')
assert.ok(lifecycleService.includes('authorization.request_canceled'), 'requester cancellation must be audited')
assert.ok(lifecycleApi.includes('outbound-authorization-requests/:request_id/cancel'), 'requester cancellation route must be registered')
assert.ok(clientApiRoute.includes('ensure_open_commerce_developer_app_owned_by_user'), 'merchant approval must recheck that the requester app is active and owned')
assert.ok(merchantWorkspace.includes('OpenCommerceRateLimitManager'), 'merchant workspace must expose external invocation quotas')
assert.ok(rateLimitManager.includes('幂等重放不重复计数'), 'quota UI must explain idempotent replay behavior')
assert.ok(rateLimitManager.includes('默认沿用现有调用行为'), 'quota UI must state the no-policy default')
assert.ok(rateLimitStore.includes('ON CONFLICT(policy_id, subject_key) DO UPDATE'), 'rate-limit claims must use a persistent atomic counter')
assert.ok(mcpTools.includes('open_commerce_upsert_rate_limit'), 'merchant AI must be able to configure invocation quotas through MCP')
assert.ok(mcpTools.includes('open_commerce_set_rate_limit_enabled'), 'merchant AI must be able to disable invocation quotas through MCP')
assert.ok(merchantWorkspace.includes('OpenCommerceAppBlockManager'), 'merchant workspace must expose emergency App blocks')
assert.ok(appBlockManager.includes('旧授权没有恢复'), 'unblocking must state that prior trust is not restored')
assert.ok(appBlockManager.includes('取消 ${outcome.canceled_authorization_requests}'), 'block result must show canceled authorization requests')
assert.ok(appBlockStore.includes("decision_reason = 'merchant_app_blocked'"), 'blocking must cancel pending requests with a stable reason')
assert.ok(appBlockStore.includes('AND revoked_at IS NULL'), 'blocking must revoke only active grants')
assert.ok(mcpTools.includes('open_commerce_block_app'), 'merchant AI must be able to block a developer App through MCP')
assert.ok(mcpTools.includes('open_commerce_unblock_app'), 'merchant AI must be able to release a developer App block through MCP')

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
