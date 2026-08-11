const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const featureRoot = 'pc-frontend/src/features/compute-supply'
const offerPanel = source(`${featureRoot}/CapacityOfferPanel.tsx`)
const panel = source(`${featureRoot}/CapacityCommitmentPanel.tsx`)
const createDialog = source(`${featureRoot}/CreateCapacityCommitmentDialog.tsx`)
const cancelDialog = source(`${featureRoot}/CancelCapacityCommitmentDialog.tsx`)
const api = source(`${featureRoot}/computeCapacityCommitmentApi.ts`)
const backendApi = source('server/src/compute_federation/capacity_commitment_api.rs')
const backendService = source('server/src/compute_federation/capacity_commitment_service.rs')
const referenceQuery = source('server/src/store/compute_platform_reference_price_curve/query.rs')

assert.match(offerPanel, /<CapacityCommitmentPanel view=\{selected\}/, 'capacity commitments must extend the selected Offer workspace')
assert.doesNotMatch(offerPanel, /navigate\(|compute-capacity-commitments/, 'the feature must not create a parallel market entry')

assert.match(api, /capacity-commitments/, 'the PC API must use the owner commitment collection')
assert.match(api, /capacity-commitment-source/, 'the PC API must resolve the governed Snapshot binding')
assert.match(api, /confirm_commitment: true/, 'capacity locking must carry explicit confirmation')
assert.match(api, /confirm_cancel: true/, 'capacity cancellation must carry explicit confirmation')
assert.doesNotMatch(`${panel}\n${createDialog}\n${cancelDialog}`, /JSON\.parse|JSON\.stringify/, 'providers must not author immutable contracts as raw JSON')

for (const field of [
  'provider_policy_revision',
  'provider_digest',
  'offer_digest',
  'pool_digest',
  'delivery_window_digest',
  'price_snapshot_digest',
  'reference_binding_digest',
  'instrument_id',
]) {
  assert.ok(createDialog.includes(field), `create dialog must derive exact ${field}`)
}
assert.match(createDialog, /specs\.map/, 'the create dialog must submit the complete meter set')
assert.match(createDialog, /quantity_units: quantities\[item\.meter\]/, 'each meter must keep its explicit quantity')
assert.match(createDialog, /available.*held/, 'the dialog must disclose the immediate capacity effect')
assert.match(panel, /detail\.current_status === 'committed'/, 'only live commitments may expose cancellation')
assert.match(panel, /delivery_window\.starts_at_utc/, 'cancellation must fail closed after delivery starts')

assert.match(backendApi, /capacity-commitment-source/, 'the authenticated source route must be registered')
assert.match(backendService, /compute_federation_price_snapshot_service::get_for_user/, 'source lookup must verify Provider ownership first')
assert.match(backendService, /platform_reference_snapshot_binding/, 'source lookup must reuse the governed binding authority')
assert.match(referenceQuery, /snapshot_binding_by_snapshot_on/, 'binding lookup must use the existing audited v223 read path')

console.log('PC capacity commitment workbench contracts passed')

function source(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), 'utf8')
}
