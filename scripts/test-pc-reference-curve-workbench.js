const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const featureRoot = 'pc-frontend/src/features/compute-reference-curves'
const app = source('pc-frontend/src/App.tsx')
const rail = source('pc-frontend/src/features/shell/ServerRail.tsx')
const page = source(`${featureRoot}/ComputeReferenceCurvePage.tsx`)
const api = source(`${featureRoot}/computeReferenceCurveApi.ts`)
const submitDialog = source(`${featureRoot}/ReferenceCurveSubmitDialog.tsx`)
const entryBuilder = source(`${featureRoot}/ReferenceCurveOfferEntryBuilder.tsx`)
const draft = source(`${featureRoot}/referenceCurveDraft.ts`)
const backendApi = source('server/src/compute_federation/platform_reference_price_curve_api.rs')
const backendService = source('server/src/compute_federation/platform_reference_price_curve_service.rs')

assert.match(app, /path="compute-reference-curves"/, 'the administrator route must be registered')
assert.match(rail, /REFERENCE_CURVE_ITEM/, 'the administrator rail must expose the workbench')
assert.match(rail, /\['admin', 'owner'\]/, 'the workbench rail item must remain role-gated')
assert.match(page, /user\?\.role === 'admin' \|\| user\?\.role === 'owner'/, 'the page must fail closed for non-admin users')

for (const suffix of ['', '/preflight', '/review', '/application']) {
  assert.ok(api.includes(suffix ? `/${suffix.slice(1)}` : "const base = '/api/admin/compute/platform-reference-price-curves'"), `PC API must preserve ${suffix || 'base'} route`)
}
assert.match(api, /reference_curve_batches\.map\(\(detail\) => detail\.batch\)/, 'list results must unwrap backend detail receipts')
assert.match(backendApi, /json!\(\{"reference_curve_batches": items\}\)/, 'backend list envelope must stay aligned with the PC client')
assert.match(backendService, /Vec<ComputePlatformReferencePriceCurveBatchDetailReceipt>/, 'backend list items must remain detail receipts')

assert.match(entryBuilder, /computeOfferAdminApi\.get/, 'entries must be built from an existing governed Offer')
assert.match(entryBuilder, /loaded\.offer\.status !== 'active'/, 'inactive Offers must fail closed')
assert.doesNotMatch(`${submitDialog}\n${entryBuilder}`, /JSON\.parse|JSON\.stringify/, 'administrators must not author price materials as raw JSON')
assert.match(draft, /fee_rules: \[\]/, 'V1 reference entries must not invent fee rules')
assert.match(draft, /Number\.isSafeInteger/, 'price arithmetic must stay inside the JavaScript safe-integer contract')
assert.match(submitDialog, /confirm_submission: true/, 'batch submission must be explicitly confirmed')
assert.match(submitDialog, /不会自动创建 Job、预留容量或移动资金/, 'the UI must state the no-market-effect boundary')
assert.match(page, /loadForStatus\('submitted', receipt\.batch_id\)/, 'successful submission must refresh the target status explicitly')
assert.match(page, /loadForStatus\('applied', detail\.batch\.batch_id\)/, 'successful application must refresh the applied queue explicitly')

assert.match(backendService, /detail\.batch\.submitted_by_admin_user_id != admin_user_id/, 'review must remain independent from submission')
assert.match(backendService, /market_effect: "none"/, 'preflight must not claim a market side effect')

console.log('PC reference curve workbench contracts passed')

function source(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), 'utf8')
}
