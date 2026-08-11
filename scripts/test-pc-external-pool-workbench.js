const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const featureRoot = 'pc-frontend/src/features/compute-external-pools'
const app = source('pc-frontend/src/App.tsx')
const rail = source('pc-frontend/src/features/shell/ServerRail.tsx')
const page = source(`${featureRoot}/ComputeExternalPoolsPage.tsx`)
const api = source(`${featureRoot}/externalPoolApi.ts`)
const draft = source(`${featureRoot}/externalPoolDraft.ts`)
const ownerDialog = source(`${featureRoot}/OnboardingSubmitDialog.tsx`)
const ownerWorkspace = source(`${featureRoot}/OnboardingWorkspace.tsx`)
const releaseDialog = source(`${featureRoot}/AdapterReleaseSubmitDialog.tsx`)
const releaseWorkspace = source(`${featureRoot}/AdapterReleaseWorkspace.tsx`)
const onboardingService = source('server/src/compute_federation/external_pool_onboarding_service.rs')
const releaseService = source('server/src/compute_federation/external_pool_adapter_release_service.rs')

assert.match(app, /path="compute-external-pools"/, 'the shared external-pool route must be registered')
assert.match(rail, /path: '\/compute-external-pools'.*label: '外部算力池'/, 'all signed-in users must be able to open their owner workspace')
assert.match(page, /role === 'admin' \|\| role === 'owner'/, 'administrator workspaces must stay role-gated')
assert.match(page, /effectiveView = isAdmin \? view : 'mine'/, 'role downgrade must fall back to the owner workspace')

for (const route of [
  '/api/me/compute/external-pool-onboarding-requests',
  '/api/admin/compute/external-pool-onboarding-requests',
  '/api/admin/compute/external-pool-adapter-releases',
]) assert.ok(api.includes(route), `PC API must preserve ${route}`)
for (const operation of ['listMine', 'preflightMine', 'cancelMine', 'reviewOnboarding', 'applyOnboarding', 'listReleases', 'preflightRelease', 'reviewRelease', 'stageRelease']) {
  assert.ok(api.includes(`${operation}:`), `PC API must expose ${operation}`)
}

const responseReceipt = between(api, 'export interface OnboardingRequestReceipt', 'export interface OnboardingReviewReceipt')
assert.doesNotMatch(responseReceipt, /non_bearer_credential_ref/, 'onboarding responses must not type a secret locator')
assert.match(responseReceipt, /credential_ref_present/, 'onboarding responses must expose presence only')
assert.match(ownerWorkspace, /已保管（不回显）/, 'the owner/admin detail must explain secret redaction')
assert.match(ownerDialog, /\^\(vault-ref\|gateway-ref\)/, 'credential submission must accept only server locator schemes')
assert.match(draft, /replace\(\/\\\.\(\\d\{3\}\)Z\$\/, '\.\$1000000Z'\)/, 'owner timestamps must use canonical UTC nanoseconds')
assert.doesNotMatch(`${ownerDialog}\n${releaseDialog}`, /JSON\.parse|JSON\.stringify|<textarea[^>]*placeholder=".*JSON/i, 'governance forms must not use raw JSON authoring')

for (const capability of ['authenticated_ack', 'authenticated_events', 'cancel_no_start', 'idempotent_commit', 'prepare', 'reconcile']) {
  assert.ok(draft.includes(`'${capability}'`), `release forms must preserve required capability ${capability}`)
}
assert.match(releaseDialog, /尚未下载、重算摘要、验签、加载工件或生成 v213 route/, 'release submission must state the staging boundary')
assert.match(releaseWorkspace, /staged admission；没有生成 Adapter registry、credential 或 route/, 'successful staging must not claim runtime authority')

assert.match(onboardingService, /current_admin_is_owner/, 'onboarding preflight must detect owner self-review')
assert.match(onboardingService, /provider_id_already_registered/, 'onboarding preflight must block Provider ID conflicts')
assert.match(onboardingService, /onboarding_effect: "none"/, 'onboarding preflight must remain side-effect free')
assert.match(releaseService, /submitted_by_admin_user_id != admin_user_id/, 'release review must remain independent')
assert.match(releaseService, /release_effect: "none"/, 'release preflight must remain side-effect free')

console.log('PC external-pool workbench contracts passed')

function between(value, start, end) { return value.slice(value.indexOf(start), value.indexOf(end)) }
function source(relativePath) { return fs.readFileSync(path.join(root, relativePath), 'utf8') }
