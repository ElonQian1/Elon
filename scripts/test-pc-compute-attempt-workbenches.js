const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const app = source('pc-frontend/src/App.tsx')
const rail = source('pc-frontend/src/features/shell/ServerRail.tsx')
const executionPage = source('pc-frontend/src/features/compute-execution/ComputeExecutionPage.tsx')
const executionQueue = source('pc-frontend/src/features/compute-execution/ProviderExecutionQueue.tsx')
const leasePanel = source('pc-frontend/src/features/compute-execution/AttemptLeasePanel.tsx')
const executionApi = source('pc-frontend/src/features/compute-execution/computeExecutionApi.ts')

const routes = [
  'compute-reviews', 'compute-execution', 'compute-observations', 'compute-verification',
  'compute-receipts', 'compute-finalization', 'compute-settlement-issuance',
  'compute-challenges', 'compute-challenge-resolution', 'compute-corrections',
  'my-compute-settlement', 'compute-settlement',
]
for (const route of routes) assert.match(app, new RegExp(`path="${route}"`), `App must register ${route}`)

for (const route of ['compute-reviews', 'compute-execution', 'compute-challenges', 'my-compute-settlement']) {
  assert.match(rail, new RegExp(`path: '/${route}'`), `${route} must remain in the signed-in participant rail`)
}
for (const constant of [
  'OBSERVATION_ITEM', 'VERIFICATION_ITEM', 'RECEIPT_ITEM', 'FINALIZATION_ITEM',
  'SETTLEMENT_ISSUANCE_ITEM', 'CHALLENGE_RESOLUTION_ITEM', 'SETTLEMENT_CORRECTION_ITEM',
  'SETTLEMENT_ITEM',
]) assert.match(rail, new RegExp(`const ${constant}`), `${constant} must remain an administrator rail item`)
assert.match(rail, /\['admin', 'owner'\]\.includes/, 'administrator workbenches must stay role-gated')

assert.doesNotMatch(executionPage, /computeExecutionApi\.activate/, 'PC must not call the failed-closed manual Start entry')
assert.doesNotMatch(leasePanel, /computeExecutionApi\.(renew|abort)/, 'PC must not call failed-closed manual Renew or Abort entries')
assert.match(executionPage, /Start、Renew 与 no-start Abort 只能由认证 Gateway 推进/, 'the Gateway boundary must be visible')
assert.match(executionQueue, /disabled title="等待认证 Gateway 接受任务"/, 'activation candidates must stay read-only')
assert.match(leasePanel, /disabled title="等待认证 Gateway 续租"/, 'manual renewal must stay disabled')
assert.match(leasePanel, /disabled title="等待认证 no-start 证明"/, 'manual no-start abort must stay disabled')

for (const fragment of [
  '/attempt-activations?limit=', '/attempt-leases?limit=', '/activation', '/state',
  '/declared-usage', '/declared-usage/latest', '/terminal-candidate',
]) assert.ok(executionApi.includes(fragment), `execution read/evidence API must retain ${fragment}`)
assert.match(leasePanel, /provider_declared/, 'declared usage must not be presented as verified')
assert.match(leasePanel, /终态候选已保存；Lease、容量和资金状态均未改变/, 'terminal declarations must preserve the no-effect boundary')

const apiContracts = [
  ['pc-frontend/src/features/compute-market/computeConsumerReviewApi.ts', 'pending-consumer-review'],
  ['pc-frontend/src/features/compute-observations/computePlatformObservationApi.ts', 'pending-platform-observation'],
  ['pc-frontend/src/features/compute-verification/computeVerificationApi.ts', 'pending-verification'],
  ['pc-frontend/src/features/compute-receipts/computeExecutionReceiptApi.ts', 'pending-execution-receipt'],
  ['pc-frontend/src/features/compute-finalization/computeAttemptFinalizationApi.ts', 'pending-trusted-finalization'],
  ['pc-frontend/src/features/compute-settlement/computeSettlementIssuanceApi.ts', 'pending-settlement-receipt'],
  ['pc-frontend/src/features/compute-settlement/computeSettlementChallengeApi.ts', 'pending-challenge'],
  ['pc-frontend/src/features/compute-settlement/computeSettlementChallengeResolutionApi.ts', 'settlement-challenges/open'],
  ['pc-frontend/src/features/compute-settlement/computeSettlementCorrectionApi.ts', 'pending-correction'],
]
for (const [file, fragment] of apiContracts) assert.ok(source(file).includes(fragment), `${file} must retain ${fragment}`)

const serverRoutes = source('server/src/compute_federation_attempt_api.rs')
  + source('server/src/compute_federation_attempt_finalization_api.rs')
  + source('server/src/compute_federation_attempt_settlement_api.rs')
  + source('server/src/compute_federation_attempt_settlement_challenge_api.rs')
  + source('server/src/compute_federation_attempt_settlement_challenge_resolution_api.rs')
  + source('server/src/compute_federation_attempt_settlement_correction_api.rs')
for (const [, fragment] of apiContracts) assert.ok(serverRoutes.includes(fragment), `server router must retain ${fragment}`)

const boundaries = [
  ['pc-frontend/src/features/compute-finalization/FinalizeAttemptDialog.tsx', '不会扣除消费者预授权'],
  ['pc-frontend/src/features/compute-settlement/IssueSettlementReceiptDialog.tsx', '不会调用银行、支付机构、钱包或 Sui'],
  ['pc-frontend/src/features/compute-settlement/ComputeSettlementChallengePage.tsx', '不会立即退款'],
  ['pc-frontend/src/features/compute-settlement/ComputeSettlementCorrectionPage.tsx', '不会发起或证明银行、钱包或链上退款'],
  ['pc-frontend/src/features/compute-settlement/ResolveSettlementChallengeDialog.tsx', 'confirm_no_money_movement'],
  ['pc-frontend/src/features/compute-settlement/WithdrawalRequestDialog.tsx', '不会立即对外付款'],
  ['pc-frontend/src/features/compute-settlement/WithdrawalTerminalDialog.tsx', 'confirm_external_payment_already_completed'],
]
for (const [file, phrase] of boundaries) assert.ok(source(file).includes(phrase), `${file} must preserve ${phrase}`)

console.log('PC compute Attempt workbench contracts passed')

function source(relativePath) { return fs.readFileSync(path.join(root, relativePath), 'utf8') }
