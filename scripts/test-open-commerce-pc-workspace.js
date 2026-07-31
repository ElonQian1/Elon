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

console.log('Open commerce PC workspace contracts passed')

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), 'utf8')
}
