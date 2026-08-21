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
const lineageContracts = source('pc-frontend/src/features/compute-attempt/federationHistoricalLineageContracts.ts')
const releaseLineageContracts = source('pc-frontend/src/features/compute-attempt/federationHistoricalReleaseLineageContracts.ts')
const verificationLineageContracts = source('pc-frontend/src/features/compute-attempt/federationHistoricalVerificationLineageContracts.ts')
const lineageApi = source('pc-frontend/src/features/compute-settlement/federationHistoricalLineageApi.ts')
const lineageButton = source('pc-frontend/src/features/compute-settlement/FederationHistoricalLineageButton.tsx')
const lineageStyles = source('pc-frontend/src/features/compute-settlement/FederationHistoricalLineageButton.module.css')
const lifecycleHistory = source('pc-frontend/src/features/compute-settlement/SettlementLifecycleHistoryList.tsx')
const challengePage = source('pc-frontend/src/features/compute-settlement/ComputeSettlementChallengePage.tsx')
const resolutionPage = source('pc-frontend/src/features/compute-settlement/ComputeSettlementChallengeResolutionPage.tsx')
const mySettlementPage = source('pc-frontend/src/features/compute-settlement/MyComputeSettlementPage.tsx')

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

assert.deepEqual(quotedConstKeys(lineageContracts, 'READ_RESPONSE_KEYS'), [
  'schema', 'lineage_kind', 'lineage_digest', 'canonical_carrier_json', 'read_effect',
], 'historical lineage read response must retain the exact five-key ABI')
assert.deepEqual(quotedConstKeys(lineageContracts, 'CARRIER_KEYS'), [
  'schema', 'lineage_kind', 'lineage_digest', 'canonicalization', 'digest_algorithm', 'lineage',
], 'historical lineage Carrier must retain the exact six-key envelope')
assert.deepEqual(quotedConstKeys(lineageContracts, 'EXECUTION_LINEAGE_KEYS'), [
  'execution_receipt', 'provider', 'capacity_pool', 'offer', 'price_snapshot', 'job',
  'reservation', 'capacity_claim', 'attempt_lease_source',
], 'execution_source_v1 must retain its exact profile')
assert.deepEqual(quotedConstKeys(lineageContracts, 'SETTLEMENT_LINEAGE_KEYS'), [
  'attempt_settlement', 'execution_receipt', 'execution_lineage_digest', 'finalization',
  'price_snapshot', 'provider', 'source_job', 'terminal_job', 'terminal_reservation',
], 'settlement_source_v1 must retain its exact profile')

const primitiveKeySets = [
  ['ProviderVersionRef', ['provider_id', 'policy_revision', 'provider_digest']],
  ['CapacityPoolVersionRef', ['pool_id', 'capacity_epoch', 'pool_revision', 'pool_digest']],
  ['OfferVersionRef', ['provider_id', 'offer_id', 'offer_version', 'offer_digest']],
  ['PriceSnapshotRef', ['price_snapshot_id', 'price_snapshot_digest']],
  ['JobVersionRef', ['job_id', 'job_revision', 'job_digest']],
  ['ReservationVersionRef', ['reservation_id', 'reservation_revision', 'reservation_digest']],
  ['CapacityClaimVersionRef', ['claim_id', 'claim_revision', 'claim_digest']],
  ['AttemptLeaseSourceRef', ['lease_id', 'lease_revision', 'lease_digest', 'fencing_generation']],
  ['ExecutionReceiptRef', ['execution_receipt_id', 'execution_receipt_digest']],
  ['FinalizationRef', ['finalization_id', 'finalization_event_digest']],
  ['AttemptSettlementRef', ['settlement_receipt_id', 'settlement_receipt_digest', 'settlement_event_digest']],
]
for (const [label, keys] of primitiveKeySets) assertExactKeyList(lineageContracts, label, keys)

assert.ok(lineageContracts.includes("const READ_SCHEMA = 'compute_federation.core_historical_causal_reference.read.v1'"), 'read schema must stay frozen')
assert.match(lineageContracts, /expectLiteral\(response\.read_effect, 'none'/, 'read_effect must be none')
assert.match(lineageContracts, /exactObject\(value, READ_RESPONSE_KEYS/, 'untrusted responses must be exact-key validated')
assert.match(lineageContracts, /expectLiteral\(carrier\.lineage_kind, response\.lineage_kind/, 'outer and inner kinds must match')
assert.match(lineageContracts, /expectLiteral\(carrier\.lineage_digest, response\.lineage_digest/, 'outer and inner digests must match')
assert.match(lineageContracts, /canonicalBytes\.byteLength !== canonicalCarrierBytes\.byteLength/, 'canonical Carrier byte lengths must match')
assert.match(lineageContracts, /canonicalBytes\.some\(\(byte, index\) => byte !== canonicalCarrierBytes\[index\]\)/, 'canonical Carrier bytes must match exactly')
assert.ok(lineageContracts.includes("canonicalizeJcs({ ...carrier, lineage_digest: '' })"), 'self digest projection must blank only lineage_digest')
assert.ok(lineageContracts.includes("CARRIER_DIGEST_DOMAIN = 'ELON-COMPUTE-CORE-HISTORICAL-LINEAGE-V1'"), 'digest domain must stay frozen')
assert.ok(lineageContracts.includes('CARRIER_DIGEST_DOMAIN}\\u0000${digestMaterial}'), 'digest material must include the domain NUL separator')
assert.match(lineageContracts, /crypto\.subtle\.digest\('SHA-256'/, 'Carrier digest must use SHA-256')
assert.match(lineageContracts, /execution\.response\.lineage_kind !== 'execution_source_v1'/, 'pair validation must require the execution response kind')
assert.match(lineageContracts, /settlement\.response\.lineage_kind !== 'settlement_source_v1'/, 'pair validation must require the settlement response kind')
assert.match(lineageContracts, /settlement\.carrier\.lineage\.execution_lineage_digest,\s*execution\.response\.lineage_digest/, 'settlement must close over the fetched execution digest')
assert.deepEqual(quotedConstKeys(releaseLineageContracts, 'LINEAGE_KEYS'), [
  'attempt_settlement', 'settlement_lineage_digest', 'source_settlement_posting',
  'release_gate', 'settlement_release', 'release_posting',
], 'release Carrier lineage keys must stay exact')
for (const gate of ['no_challenge', 'resolved_challenge', 'accepted_corrected']) {
  assert.ok(releaseLineageContracts.includes(`'${gate}'`), `release Carrier must freeze ${gate}`)
}
assert.match(releaseLineageContracts, /release\.carrier\.lineage\.settlement_lineage_digest,\s*settlement\.response\.lineage_digest/, 'release must close over the fetched settlement digest')
assert.ok(releaseLineageContracts.includes('actual.settlement_receipt_id'), 'release must bind the settlement receipt ID')
assert.ok(releaseLineageContracts.includes('actual.settlement_receipt_digest'), 'release must bind the settlement receipt digest')
assert.ok(releaseLineageContracts.includes('actual.settlement_event_digest'), 'release must bind the settlement event digest')
assert.deepEqual(quotedConstKeys(verificationLineageContracts, 'READ_KEYS'), [
  'schema', 'lineage_kind', 'lineage_digest', 'canonical_carrier_json', 'read_effect',
], 'execution verification read response must retain the exact five-key ABI')
assert.deepEqual(quotedConstKeys(verificationLineageContracts, 'CARRIER_KEYS'), [
  'schema', 'lineage_kind', 'lineage_digest', 'canonicalization', 'digest_algorithm', 'lineage',
], 'execution verification Carrier must retain the exact six-key envelope')
assert.deepEqual(quotedConstKeys(verificationLineageContracts, 'LINEAGE_KEYS'), [
  'execution_receipt', 'execution_lineage_digest', 'provider_declared_usage',
  'terminal_candidate', 'consumer_review', 'platform_observation', 'verification_decision',
], 'execution verification Carrier lineage keys must stay exact')
const verificationPrimitiveKeySets = [
  ['ExecutionReceiptRef', ['execution_receipt_id', 'execution_receipt_digest']],
  ['ProviderDeclaredUsageRef', ['usage_snapshot_id', 'usage_sequence_no', 'cumulative_usage_digest', 'usage_event_digest']],
  ['TerminalCandidateRef', ['terminal_candidate_id', 'terminal_candidate_event_digest']],
  ['ConsumerReviewRef', ['consumer_review_id', 'consumer_review_event_digest']],
  ['PlatformObservationRef', ['platform_observation_id', 'platform_observation_event_digest', 'cumulative_observed_usage_digest']],
  ['VerificationDecisionRef', ['verification_decision_id', 'verification_event_digest', 'verified_usage_digest', 'compensable_usage_digest']],
]
for (const [label, keys] of verificationPrimitiveKeySets) assertExactKeyList(verificationLineageContracts, label, keys)
assert.match(verificationLineageContracts, /expectLiteral\(response\.read_effect, ["']none["']/, 'verification read_effect must be none')
assert.match(verificationLineageContracts, /expectLiteral\(\s*response\.lineage_kind,\s*["']execution_verification_source_v1["']/, 'verification response must retain the frozen lineage kind')
assert.match(verificationLineageContracts, /expectLiteral\(\s*carrier\.lineage_kind,\s*["']execution_verification_source_v1["']/, 'verification Carrier must retain the frozen lineage kind')
assert.match(verificationLineageContracts, /carrier\.lineage_kind,\s*response\.lineage_kind/, 'verification outer and inner kinds must match')
assert.match(verificationLineageContracts, /expectLiteral\(carrier\.lineage_digest, responseDigest/, 'verification outer and inner digests must match')
assert.match(verificationLineageContracts, /canonicalBytes\.byteLength !== inputBytes\.byteLength/, 'verification canonical Carrier byte lengths must match')
assert.match(verificationLineageContracts, /canonicalBytes\.some\(\(byte, index\) => byte !== inputBytes\[index\]\)/, 'verification canonical Carrier bytes must match exactly')
assert.match(verificationLineageContracts, /canonicalizeJcs\(\{ \.\.\.carrier, lineage_digest: ["']["'] \}\)/, 'verification self digest projection must blank only lineage_digest')
assert.ok(verificationLineageContracts.includes('`${CARRIER_DIGEST_DOMAIN}\\u0000${material}`'), 'verification digest material must include the frozen domain NUL separator')
assert.match(verificationLineageContracts, /await sha256Hex\(/, 'verification Carrier digest must use SHA-256')
assert.match(verificationLineageContracts, /verification\.carrier\.lineage\.execution_lineage_digest,\s*execution\.response\.lineage_digest/, 'verification must close over the fetched execution digest')
assert.match(verificationLineageContracts, /verification\.carrier\.lineage\.execution_receipt\.execution_receipt_id,\s*execution\.carrier\.lineage\.execution_receipt\.execution_receipt_id/, 'verification must bind the execution receipt ID')
assert.match(verificationLineageContracts, /verification\.carrier\.lineage\.execution_receipt\.execution_receipt_digest,\s*execution\.carrier\.lineage\.execution_receipt\.execution_receipt_digest/, 'verification must bind the execution receipt digest')

assert.ok(lineageApi.includes("participant: '/api/me/compute/attempt-leases'"), 'participant reads must use the /api/me scope')
assert.ok(lineageApi.includes("admin: '/api/admin/compute/attempt-leases'"), 'admin reads must use the /api/admin scope')
assert.ok(lineageApi.includes('execution-source-lineage'), 'execution lineage GET suffix must stay frozen')
assert.ok(lineageApi.includes('settlement-source-lineage'), 'settlement lineage GET suffix must stay frozen')
assert.ok(lineageApi.includes('settlement-release-source-lineage'), 'release lineage GET suffix must stay frozen')
assert.ok(lineageApi.includes('execution-verification-source-lineage'), 'verification lineage GET suffix must stay frozen')
assert.match(lineageApi, /api\.get<unknown>/, 'lineage HTTP payloads must remain untrusted until runtime validation')
assert.doesNotMatch(lineageApi, /api\.(?:post|patch|put|delete)/, 'historical lineage adoption must remain read-only')
assert.match(lineageButton, /await Promise\.all\(\[\s*federationHistoricalLineageApi\.readExecution[\s\S]*federationHistoricalLineageApi\.readSettlement/, 'execution and settlement lineage must be fetched in parallel')
assert.match(lineageButton, /federationHistoricalLineageApi\.readExecution[\s\S]*federationHistoricalLineageApi\.readVerification[\s\S]*federationHistoricalLineageApi\.readSettlement/, 'execution, verification, and settlement lineage must be fetched in one parallel batch')
assert.match(lineageButton, /releaseAvailable\s*\? federationHistoricalLineageApi\.readRelease[\s\S]*: Promise\.resolve\(null\)/, 'pending rows must not turn an absent release into integrity failure')
assert.match(lineageButton, /useLayoutEffect\(\(\) => \{[\s\S]*requestGeneration\.current \+= 1[\s\S]*setState\(\{ status: 'idle' \}\)[\s\S]*\}, \[leaseId, releaseAvailable, scope\]\)/, 'Lease, release, or scope changes must invalidate and clear historical lineage evidence before paint')
assert.equal((lineageButton.match(/if \(generation !== requestGeneration\.current\) return/g) ?? []).length, 2, 'stale lineage requests must be rejected before both success and error updates')
assert.match(lineageButton, /finally \{\s*if \(generation === requestGeneration\.current\) busy\.current = false\s*\}/, 'only the current lineage request may release the busy guard')
const pairValidationIndex = lineageButton.indexOf('validateFederationHistoricalLineagePair(execution, settlement)')
const tripleValidationIndex = lineageButton.indexOf('validateFederationHistoricalLineageTriple(execution, settlement, release)')
const verificationValidationIndex = lineageButton.indexOf('validateExecutionVerificationLineagePair(execution, verification)')
const lineageSuccessIndex = lineageButton.indexOf("setState({ status: 'success'")
assert.ok(pairValidationIndex >= 0 && lineageSuccessIndex > pairValidationIndex, 'the pair equation must pass before any evidence is displayed')
assert.ok(tripleValidationIndex >= 0 && lineageSuccessIndex > tripleValidationIndex, 'the triple equation must pass before released evidence is displayed')
assert.ok(verificationValidationIndex >= 0 && lineageSuccessIndex > verificationValidationIndex, 'the verification equation must pass before any evidence is displayed')
for (const phrase of ['并行核验因果链', '重试因果链核验', '三响应摘要与两级跨链等式已核验', '四响应摘要与三级跨链等式已核验']) {
  assert.ok(lineageButton.includes(phrase), `lineage button must expose ${phrase}`)
}
assert.match(lineageButton, /^function LineageEvidence/m, 'evidence rendering must use a module-scope component')
assert.doesNotMatch(lineageButton, /import\(/, 'lineage adoption must use direct imports')
assert.doesNotMatch(lineageButton, /localStorage|sessionStorage|download|href=/, 'lineage evidence must not be persisted or exported')
assert.doesNotMatch(releaseLineageContracts, /localStorage|sessionStorage|download|href=/, 'release lineage evidence must not be persisted or exported')
assert.doesNotMatch(verificationLineageContracts, /localStorage|sessionStorage|download|href=/, 'verification lineage evidence must not be persisted or exported')
assert.match(releaseLineageContracts, /\\u007f-\\u009f/, 'release lineage IDs must reject the full C0, DEL, and C1 control ranges')
assert.match(releaseLineageContracts, /const RUST_TRIM_EDGE/, 'release lineage IDs must mirror Rust Unicode trim semantics without rejecting U+FEFF')
assert.doesNotMatch(releaseLineageContracts, /result !== result\.trim\(\)/, 'JavaScript-only FEFF trim semantics must not narrow the frozen Rust ABI')
assert.match(verificationLineageContracts, /\\u007f-\\u009f/, 'verification lineage IDs must reject the full C0, DEL, and C1 control ranges')
assert.match(verificationLineageContracts, /const RUST_TRIM_EDGE/, 'verification lineage IDs must mirror Rust Unicode trim semantics without rejecting U+FEFF')
assert.doesNotMatch(verificationLineageContracts, /result !== result\.trim\(\)/, 'verification lineage must not apply JavaScript-only FEFF trimming')
assert.match(lineageStyles, /overflow-wrap:\s*anywhere/, 'long historical digests must stay readable')
assert.match(lifecycleHistory, /FederationHistoricalLineageButton[\s\S]*leaseId=\{item\.settlement\.lease_id\}[\s\S]*scope=\{scope\}[\s\S]*releaseAvailable=\{Boolean\(item\.release\)\}/, 'each settlement history row must pass its Lease, scope, and release presence')
assert.match(challengePage, /SettlementLifecycleHistoryList items=\{history\} loading=\{loading\} scope="participant"/, 'consumer challenge history must use participant scope')
assert.match(resolutionPage, /SettlementLifecycleHistoryList items=\{history\} loading=\{loading\} scope="admin"/, 'resolution history must use admin scope')
assert.match(mySettlementPage, /SettlementLifecycleHistoryList items=\{settlementHistory\} loading=\{loadingAccount\} scope="participant"/, 'my settlement history must use participant scope')
assert.match(mySettlementPage, /const accountRequestGeneration = useRef\(0\)/, 'Provider settlement history must own a request generation')
assert.match(mySettlementPage, /useLayoutEffect\(\(\) => \{ void loadAccount\(\) \}, \[loadAccount\]\)/, 'Provider changes must clear stale settlement evidence before paint')
assert.match(mySettlementPage, /if \(accountRequestKey\.current\.providerId !== requestProviderId[\s\S]*accountRequestKey\.current\.status !== requestStatus\) return[\s\S]*const generation = \+\+accountRequestGeneration\.current/, 'a stale callback must not clear or reload the current Provider view')
assert.match(mySettlementPage, /if \(!isCurrentRequest\(\)\) return[\s\S]*setSettlementHistory\(nextSettlementHistory\)/, 'stale Provider settlement history must not replace the current Provider view')
assert.doesNotMatch(app + rail, /federation-historical-lineage|source-lineage/i, 'historical lineage adoption must not add a route or navigation item')

console.log('PC compute Attempt workbench contracts passed')

function source(relativePath) { return fs.readFileSync(path.join(root, relativePath), 'utf8') }

function quotedConstKeys(content, name) {
  const match = content.match(new RegExp(`const ${name} = \\[([\\s\\S]*?)\\] as const`))
  assert.ok(match, `${name} must remain an as-const key list`)
  return [...match[1].matchAll(/['"]([^'"]+)['"]/g)].map((entry) => entry[1])
}

function assertExactKeyList(content, label, keys) {
  const pattern = keys.map((key) => `['"]${escapeRegex(key)}['"]`).join('\\s*,\\s*')
  assert.match(content, new RegExp(`\\[\\s*${pattern}\\s*\\]`), `${label} must retain its exact key shape`)
}

function escapeRegex(value) { return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') }
