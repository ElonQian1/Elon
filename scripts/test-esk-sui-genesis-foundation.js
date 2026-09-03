const assert = require('node:assert/strict')
const crypto = require('node:crypto')
const fs = require('node:fs')
const path = require('node:path')
const root = path.resolve(__dirname, '..')
const fromRoot = (relative) => path.join(root, relative)
const read = (relative) => fs.readFileSync(fromRoot(relative), 'utf8')
const parse = (relative) => JSON.parse(read(relative))
const sha256 = (bytes) => `sha256:${crypto.createHash('sha256').update(bytes).digest('hex')}`
const canonicalTextBytes = (bytes) => Buffer.from(bytes.toString('utf8').replace(/\r\n?/g, '\n'), 'utf8')
const canonicalTextSha256 = (bytes) => sha256(canonicalTextBytes(bytes))
const schemaPath = 'contracts/sui/esk-genesis-manifest-v1.schema.json'
const defaultManifestPath = 'contracts/sui/esk-genesis-manifest-v1.fixture.json'
const moveTomlPath = 'contracts/sui/esk_currency/Move.toml'
const moveSourcePath = 'contracts/sui/esk_currency/sources/esk.move'
const moveTestsPath = 'contracts/sui/esk_currency/tests/esk_tests.move'
const requirementPath = 'docs/requirements/esk-sui-genesis-foundation-v1.md'
const decisionPath = 'docs/decisions/esk-sui-economic-foundation-v1.md'
const manifestFile = process.argv[2] ? path.resolve(process.cwd(), process.argv[2]) : fromRoot(defaultManifestPath)
const predecessorFiles = process.argv.slice(3).map((file) => path.resolve(process.cwd(), file))
const isDefaultManifest = path.resolve(manifestFile) === path.resolve(fromRoot(defaultManifestPath))
const schema = parse(schemaPath)
const manifest = JSON.parse(fs.readFileSync(manifestFile, 'utf8'))
let predecessorContext = null
for (let index = predecessorFiles.length - 1; index >= 0; index -= 1) {
  const bytes = fs.readFileSync(predecessorFiles[index])
  predecessorContext = { manifest: JSON.parse(bytes.toString('utf8')), digest: canonicalTextSha256(bytes), predecessor: predecessorContext }
}
const moveToml = read(moveTomlPath)
const moveSource = read(moveSourcePath)
const moveTests = read(moveTestsPath)
const requirement = read(requirementPath)
const decision = read(decisionPath)
const actualMoveSourceDigest = canonicalTextSha256(fs.readFileSync(fromRoot(moveSourcePath)))
const actualMoveTestDigest = canonicalTextSha256(fs.readFileSync(fromRoot(moveTestsPath)))
const suiDependencyBlock = moveToml.match(/\bSui\s*=\s*\{([\s\S]*?)\}/)?.[1]
const pinnedSuiRev = suiDependencyBlock?.match(/\brev\s*=\s*"([0-9a-f]{40})"/)?.[1]
function movePackageInputDigest() {
  const packageRoot = fromRoot('contracts/sui/esk_currency')
  const files = []
  const visit = (directory) => fs.readdirSync(directory, { withFileTypes: true }).forEach((entry) => {
    const absolute = path.join(directory, entry.name)
    if (entry.isDirectory() && entry.name !== 'build') visit(absolute)
    else if (entry.isFile() && (entry.name === 'Move.toml' || entry.name === 'Move.lock' || entry.name.endsWith('.move'))) files.push(absolute)
  })
  visit(packageRoot)
  const hash = crypto.createHash('sha256')
  files.sort().forEach((file) => hash.update(path.relative(packageRoot, file).replaceAll('\\', '/')).update('\0').update(canonicalTextBytes(fs.readFileSync(file))).update('\0'))
  return `sha256:${hash.digest('hex')}`
}
const actualPackageInputDigest = movePackageInputDigest()
const publishedStates = new Set(['testnet_published', 'mainnet_published'])
const holderRequiredStates = new Set(['testnet_published', 'mainnet_ready', 'mainnet_published'])
const mainnetControlledStates = new Set(['mainnet_ready', 'mainnet_published'])
const chainEvidenceKeys = ['chain_identifier', 'publication_endpoint_ref', 'package_id', 'type_tag', 'pending_currency_object_id', 'registered_currency_object_id', 'metadata_cap_object_id', 'upgrade_cap_object_id', 'initial_supply_coin_object_id', 'publish_tx_digest', 'currency_registration_tx_digest', 'allocation_tx_digest', 'role_handoff_tx_digest', 'publish_checkpoint', 'registration_checkpoint', 'verify_source_result_digest']
const expectedRoles = ['deployer', 'treasury', 'upgrade', 'metadata', 'distribution', 'pause', 'gas_sponsor', 'team_vesting', 'liquidity']
const multisigRoles = ['treasury', 'upgrade', 'metadata', 'distribution', 'pause', 'team_vesting', 'liquidity']
const operationalRoles = ['treasury', 'upgrade', 'metadata', 'distribution', 'pause', 'gas_sponsor', 'team_vesting', 'liquidity']
const expectedClaimFields = ['opaque_subject', 'reconciled_payment_asset', 'reconciled_payment_amount', 'external_payment_reference', 'payment_observed_at', 'commercial_purpose', 'sale_batch_id', 'esk_base_units', 'disclosure_revision', 'eligibility_decision', 'address_control_proof', 'idempotency_key', 'approval_digest']
const expectedLedgerAuthorities = { esk_balance: 'yilong_main', esk_onchain_ownership: 'sui', esk_profit_distribution: 'yilong_main', service_order: 'yilong_main', project_treasury: 'yilong_main', qshare_nav: 'yilong_quant', customer_quant_principal: 'yilong_quant', legal_equity_register: 'legal_entity_register' }

function deepEqual(left, right) {
  try {
    assert.deepEqual(left, right)
    return true
  } catch {
    return false
  }
}

function resolveRef(ref) {
  assert.match(ref, /^#\/\$defs\/[A-Za-z0-9_-]+$/, `unsupported local schema ref: ${ref}`)
  return ref.slice(8).split('/').reduce((value, part) => value[part], schema.$defs)
}

function typeMatches(expected, value) {
  if (expected === 'null') return value === null
  if (expected === 'array') return Array.isArray(value)
  if (expected === 'object') return value !== null && typeof value === 'object' && !Array.isArray(value)
  if (expected === 'integer') return Number.isInteger(value)
  return typeof value === expected
}

function isStrictRfc3339(value) {
  if (typeof value !== 'string') return false
  const match = value.match(/^([0-9]{4})-(0[1-9]|1[0-2])-([0-2][0-9]|3[01])T([01][0-9]|2[0-3]):([0-5][0-9]):([0-5][0-9])(\.[0-9]+)?(Z|[+-]([01][0-9]|2[0-3]):[0-5][0-9])$/)
  if (!match) return false
  const year = Number(match[1])
  const month = Number(match[2])
  const day = Number(match[3])
  if (year === 0) return false
  const leap = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0)
  const days = [31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
  return day <= days[month - 1] && Number.isFinite(Date.parse(value))
}

function validate(node, value, location = '$') {
  const errors = []
  const add = (message) => errors.push(`${location}: ${message}`)
  if (node.$ref) errors.push(...validate(resolveRef(node.$ref), value, location))
  const types = node.type === undefined ? [] : Array.isArray(node.type) ? node.type : [node.type]

  if (types.length && !types.some((type) => typeMatches(type, value))) {
    add(`expected type ${types.join('|')}`)
    return errors
  }
  if (Object.hasOwn(node, 'const') && !deepEqual(value, node.const)) add(`must equal ${JSON.stringify(node.const)}`)
  if (node.enum && !node.enum.some((candidate) => deepEqual(value, candidate))) add('must match enum')

  if (typeof value === 'string') {
    if (node.minLength !== undefined && value.length < node.minLength) add(`must contain at least ${node.minLength} characters`)
    if (node.maxLength !== undefined && value.length > node.maxLength) add(`must contain at most ${node.maxLength} characters`)
    if (node.pattern && !new RegExp(node.pattern).test(value)) add(`must match ${node.pattern}`)
    if (node.format === 'date-time' && !isStrictRfc3339(value)) add('must be a strict RFC3339 date-time')
    if (node.format === 'uri') {
      try {
        new URL(value)
      } catch {
        add('must be an absolute URI')
      }
    }
  }
  if (typeof value === 'number') {
    if (node.minimum !== undefined && value < node.minimum) add(`must be >= ${node.minimum}`)
    if (node.maximum !== undefined && value > node.maximum) add(`must be <= ${node.maximum}`)
  }
  if (Array.isArray(value)) {
    if (node.minItems !== undefined && value.length < node.minItems) add(`must contain at least ${node.minItems} items`)
    if (node.maxItems !== undefined && value.length > node.maxItems) add(`must contain at most ${node.maxItems} items`)
    if (node.uniqueItems) {
      const serialized = value.map((item) => JSON.stringify(item))
      if (new Set(serialized).size !== serialized.length) add('must contain unique items')
    }
    if (node.items) value.forEach((item, index) => errors.push(...validate(node.items, item, `${location}[${index}]`)))
  }
  if (value !== null && typeof value === 'object' && !Array.isArray(value)) {
    for (const required of node.required || []) {
      if (!Object.hasOwn(value, required)) add(`missing required property ${required}`)
    }
    for (const [key, child] of Object.entries(node.properties || {})) {
      if (Object.hasOwn(value, key)) errors.push(...validate(child, value[key], `${location}.${key}`))
    }
    if (node.additionalProperties === false) {
      for (const key of Object.keys(value)) {
        if (!Object.hasOwn(node.properties || {}, key)) add(`unexpected property ${key}`)
      }
    }
  }
  for (const condition of node.allOf || []) {
    if (!condition.if) {
      errors.push(...validate(condition, value, location))
      continue
    }
    const ifErrors = validate(condition.if, value, location)
    if (ifErrors.length === 0) errors.push(...validate(condition.then, value, location))
    else if (condition.else) errors.push(...validate(condition.else, value, location))
  }
  return errors
}

function clone(value) {
  return JSON.parse(JSON.stringify(value))
}

function sameMembers(actual, expected) {
  return Array.isArray(actual) && actual.length === expected.length &&
    deepEqual([...actual].sort(), [...expected].sort())
}

function semanticErrors(candidate, predecessor = null) {
  const errors = []
  const add = (location, message) => errors.push(`${location}: ${message}`)
  const lifecycle = candidate.lifecycle || {}
  const network = candidate.chain?.network
  const state = lifecycle.state
  const published = publishedStates.has(state)

  if (!isStrictRfc3339(lifecycle.updated_at)) add('$.lifecycle.updated_at', 'must be a real RFC3339 calendar date')
  if (lifecycle.revision === 1) {
    if (predecessor) add('$.lifecycle', 'revision 1 cannot be supplied with a predecessor file')
    for (const key of ['predecessor_manifest_id', 'predecessor_revision', 'predecessor_digest']) {
      if (lifecycle[key] !== null) add(`$.lifecycle.${key}`, 'revision 1 cannot claim a predecessor')
    }
  } else if (Number.isInteger(lifecycle.revision) && lifecycle.revision > 1) {
    if (lifecycle.predecessor_revision !== lifecycle.revision - 1) {
      add('$.lifecycle.predecessor_revision', 'must equal revision minus one')
    }
    if (lifecycle.predecessor_manifest_id !== candidate.manifest_id || typeof lifecycle.predecessor_digest !== 'string') {
      add('$.lifecycle', 'later revisions require the same manifest id plus predecessor revision and digest')
    }
    if (!predecessor) add('$.lifecycle', 'later revisions require predecessor manifest files as trailing CLI arguments, newest first')
    else {
      const prior = predecessor.manifest
      if (prior.manifest_id !== candidate.manifest_id || prior.lifecycle?.revision !== lifecycle.predecessor_revision || predecessor.digest !== lifecycle.predecessor_digest) add('$.lifecycle', 'predecessor file identity, revision, and exact byte digest must match')
      const priorErrors = allErrors(prior, predecessor.predecessor)
      if (priorErrors.length) add('$.lifecycle', `predecessor manifest must validate independently: ${priorErrors.join(' | ')}`)
      if (!isStrictRfc3339(prior.lifecycle?.updated_at) || Date.parse(prior.lifecycle.updated_at) >= Date.parse(lifecycle.updated_at)) add('$.lifecycle.updated_at', 'must be later than the predecessor timestamp')
      const transitions = { testnet: { planned: 'local_verified', local_verified: 'testnet_published' }, mainnet: { planned: 'local_verified', local_verified: 'mainnet_ready', mainnet_ready: 'mainnet_published' } }
      if (transitions[candidate.chain?.network]?.[prior.lifecycle?.state] !== state) add('$.lifecycle.state', 'must be the next legal state after the predecessor')
      for (const field of ['schema', 'manifest_id', 'asset', 'chain', 'supply', 'allocations', 'ledger_boundaries']) {
        if (!deepEqual(prior[field], candidate[field])) add(`$.${field}`, 'must remain immutable across lifecycle revisions')
      }
      for (const field of ['sui_source_commit', 'package_input_digest', 'move_source_digest', 'move_test_source_digest']) {
        if (prior.toolchain?.[field] !== candidate.toolchain?.[field]) add(`$.toolchain.${field}`, 'must remain bound to the verified package across revisions')
      }
      for (const roleName of expectedRoles) {
        const publicPolicy = (role) => role && [role.role_id, role.custody, role.minimum_approvals]
        if (!deepEqual(publicPolicy(prior.roles?.[roleName]), publicPolicy(candidate.roles?.[roleName]))) add(`$.roles.${roleName}`, 'role policy cannot change across lifecycle revisions')
      }
    }
  }

  const statesByNetwork = {
    testnet: new Set(['planned', 'local_verified', 'testnet_published']),
    mainnet: new Set(['planned', 'local_verified', 'mainnet_ready', 'mainnet_published']),
  }
  if (!statesByNetwork[network]?.has(state)) add('$.lifecycle.state', `is invalid for ${network || 'unknown'} network`)
  if (network === 'mainnet') add('$.chain.network', 'this V1 implementation is pinned to a testnet release; mainnet requires a separately approved package revision and validator')
  if (state === 'mainnet_ready') add('$.lifecycle.state', 'mainnet readiness requires the governed evidence verifier, which is not implemented by this offline validator')
  if (typeof candidate.toolchain?.sui_release === 'string' && !candidate.toolchain.sui_release.startsWith(`${network}-v`)) {
    add('$.toolchain.sui_release', 'must use the manifest network release channel')
  }
  if (!pinnedSuiRev || candidate.toolchain?.sui_source_commit !== pinnedSuiRev) {
    add('$.toolchain.sui_source_commit', 'must equal the Move.toml pinned Sui revision')
  }
  const sourceBindingRequired = state !== 'planned' || candidate.toolchain?.verification_status === 'verified'
  if ((sourceBindingRequired || candidate.toolchain?.package_input_digest !== null) &&
      candidate.toolchain?.package_input_digest !== actualPackageInputDigest) {
    add('$.toolchain.package_input_digest', 'does not match Move.toml, Move.lock, and all Move package sources')
  }
  if ((sourceBindingRequired || candidate.toolchain?.move_source_digest !== null) &&
      candidate.toolchain?.move_source_digest !== actualMoveSourceDigest) {
    add('$.toolchain.move_source_digest', 'does not match the current Move source bytes')
  }
  if ((sourceBindingRequired || candidate.toolchain?.move_test_source_digest !== null) &&
      candidate.toolchain?.move_test_source_digest !== actualMoveTestDigest) {
    add('$.toolchain.move_test_source_digest', 'does not match the current Move test bytes')
  }

  let totalBaseUnits
  try {
    const scale = 10n ** BigInt(candidate.asset.decimals)
    totalBaseUnits = BigInt(candidate.supply.base_units)
    if (BigInt(candidate.supply.display_units) * scale !== totalBaseUnits) {
      add('$.supply', 'display units, decimals, and base units do not conserve supply')
    }
  } catch {
    add('$.supply', 'supply values must be valid integers')
  }
  if (Array.isArray(candidate.allocations) && totalBaseUnits !== undefined) {
    const bucketIds = candidate.allocations.map((item) => item.bucket_id)
    if (new Set(bucketIds).size !== bucketIds.length) add('$.allocations.bucket_id', 'bucket ids must be unique')
    try {
      const allocationTotal = candidate.allocations.reduce((sum, item) => sum + BigInt(item.base_units), 0n)
      const basisPointTotal = candidate.allocations.reduce((sum, item) => sum + item.basis_points, 0)
      if (allocationTotal !== totalBaseUnits) add('$.allocations.base_units', 'allocation total must equal fixed supply')
      if (basisPointTotal !== 10_000) add('$.allocations.basis_points', 'basis points must total 10000')
      candidate.allocations.forEach((item, index) => {
        if (BigInt(item.base_units) * 10_000n !== totalBaseUnits * BigInt(item.basis_points)) {
          add(`$.allocations[${index}]`, 'allocation proportion must exactly match basis points')
        }
      })
    } catch {
      add('$.allocations', 'allocation quantities must be valid integers')
    }
  }

  const roles = candidate.roles || {}
  if (!sameMembers(Object.keys(roles), expectedRoles)) add('$.roles', 'must contain the exact role set')
  const roleIds = Object.values(roles).map((role) => role?.role_id)
  if (new Set(roleIds).size !== roleIds.length) add('$.roles.role_id', 'role ids must be unique')
  for (const roleName of multisigRoles) {
    const role = roles[roleName]
    if (role?.custody !== 'multisig_required' || role?.minimum_approvals < 2) {
      add(`$.roles.${roleName}`, 'must use multisig custody with at least two approvals')
    }
  }
  if (network === 'testnet' && (roles.deployer?.custody !== 'ephemeral_deployer' || roles.deployer?.minimum_approvals !== 1)) {
    add('$.roles.deployer', 'testnet deployer must be ephemeral 1-of-1')
  }
  if (network === 'mainnet' && (roles.deployer?.custody !== 'multisig_required' || roles.deployer?.minimum_approvals < 2)) {
    add('$.roles.deployer', 'mainnet deployer must use multisig custody')
  }
  if (roles.gas_sponsor?.custody !== 'service_principal' || roles.gas_sponsor?.minimum_approvals !== 1) {
    add('$.roles.gas_sponsor', 'gas sponsor must be an isolated service principal')
  }
  if (holderRequiredStates.has(state)) {
    for (const roleName of operationalRoles) {
      if (typeof roles[roleName]?.holder_ref !== 'string') add(`$.roles.${roleName}.holder_ref`, 'must be populated before release')
    }
  }
  if (mainnetControlledStates.has(state) && typeof roles.deployer?.holder_ref !== 'string') {
    add('$.roles.deployer.holder_ref', 'must be populated before mainnet approval')
  }
  const populatedHolders = Object.values(roles).map((role) => role?.holder_ref).filter((holder) => typeof holder === 'string')
  if (new Set(populatedHolders).size !== populatedHolders.length) add('$.roles.holder_ref', 'populated holders must be isolated and unique')

  const ledgers = Array.isArray(candidate.ledger_boundaries) ? candidate.ledger_boundaries : []
  const actualLedgerAuthorities = Object.fromEntries(ledgers.map((item) => [item.ledger_id, item.authority]))
  if (!deepEqual(actualLedgerAuthorities, expectedLedgerAuthorities) || new Set(ledgers.map((item) => item.ledger_id)).size !== ledgers.length) {
    add('$.ledger_boundaries', 'must preserve the exact ledger ownership map')
  }
  if (!ledgers.every((item) => item.may_revalue_esk === false)) add('$.ledger_boundaries', 'no ledger may revalue ESK')

  const migration = candidate.paid_user_migration || {}
  if (!sameMembers(migration.required_claim_fields, expectedClaimFields)) {
    add('$.paid_user_migration.required_claim_fields', 'must contain exactly the 13 reconciliation fields')
  }
  if (!sameMembers(migration.allowed_purposes, ['esk_purchase', 'service_purchase', 'qshare_subscription'])) {
    add('$.paid_user_migration.allowed_purposes', 'must contain the three isolated commercial purposes')
  }
  if (migration.claims_enabled && (migration.claim_schema_status !== 'enabled' || !published)) {
    add('$.paid_user_migration.claims_enabled', 'requires an enabled schema and published chain state')
  }
  if (migration.claim_schema_status === 'enabled' && !migration.claims_enabled) {
    add('$.paid_user_migration.claim_schema_status', 'enabled schema requires claims_enabled')
  }

  const assurances = candidate.assurances || {}
  const falseBoundaries = ['permanent_usdt_peg', 'guaranteed_yield', 'unlimited_loss_guarantee', 'legal_equity', 'automatic_paper_conversion', 'customer_funds_moved']
  for (const boundary of falseBoundaries) {
    if (assurances[boundary] !== false) add(`$.assurances.${boundary}`, 'must fail closed')
  }
  const expectedPublicationFlag = published
  if (assurances.publication_transactions_signed !== expectedPublicationFlag) {
    add('$.assurances.publication_transactions_signed', `must be ${expectedPublicationFlag} for this lifecycle state`)
  }
  if (assurances.publication_transactions_broadcast !== expectedPublicationFlag) {
    add('$.assurances.publication_transactions_broadcast', `must be ${expectedPublicationFlag} for this lifecycle state`)
  }

  const evidence = candidate.evidence || {}
  if (published) {
    add('$.lifecycle.state', 'published manifests require the online chain verifier, which is not implemented by this offline validator')
    const receipts = evidence.allocation_receipts
    if (!Array.isArray(receipts)) add('$.evidence.allocation_receipts', 'published state requires one receipt per allocation bucket')
    else {
      const receiptIds = receipts.map((receipt) => receipt.bucket_id)
      const objectIds = receipts.map((receipt) => receipt.resulting_object_id)
      if (new Set(receiptIds).size !== receipts.length || new Set(objectIds).size !== receipts.length) add('$.evidence.allocation_receipts', 'bucket and resulting object ids must be unique')
      for (const allocation of candidate.allocations || []) {
        const receipt = receipts.find((item) => item.bucket_id === allocation.bucket_id)
        if (!receipt || receipt.base_units !== allocation.base_units || receipt.recipient_role !== allocation.recipient_role ||
            receipt.vesting_policy_ref !== allocation.vesting_policy_ref || receipt.recipient_holder_ref !== roles[allocation.recipient_role]?.holder_ref ||
            receipt.tx_digest !== evidence.allocation_tx_digest) add(`$.evidence.allocation_receipts.${allocation.bucket_id}`, 'must reproduce allocation, policy, role holder, and transaction')
        if (allocation.bucket_id === 'team_vesting' && receipt?.enforcement !== 'move_contract') add('$.evidence.allocation_receipts.team_vesting', 'team vesting must be enforced by a Move contract')
      }
    }
    for (const key of chainEvidenceKeys) {
      if (typeof evidence[key] !== 'string') add(`$.evidence.${key}`, 'published state requires concrete evidence')
    }
    if (evidence.verify_source_status !== 'verified') add('$.evidence.verify_source_status', 'published source must be verified')
    const independent = evidence.independent_verification || {}
    if (independent.status !== 'verified' || !['grpc', 'graphql'].includes(independent.transport) || typeof independent.endpoint_ref !== 'string' ||
        !isStrictRfc3339(independent.observed_at) || typeof independent.evidence_digest !== 'string') {
      add('$.evidence.independent_verification', 'published state requires independent verified chain evidence')
    }
    if (independent.endpoint_ref === evidence.publication_endpoint_ref) add('$.evidence.independent_verification.endpoint_ref', 'must differ from the publication endpoint')
  } else {
    if (evidence.allocation_receipts !== null) add('$.evidence.allocation_receipts', 'unpublished state must leave allocation receipts null')
    for (const key of chainEvidenceKeys) {
      if (evidence[key] !== null) add(`$.evidence.${key}`, 'unpublished state must leave all chain evidence null')
    }
    if (evidence.verify_source_status !== 'not_run') add('$.evidence.verify_source_status', 'unpublished state cannot claim source verification')
    const independent = evidence.independent_verification || {}
    if (!deepEqual(independent, { status: 'not_run', transport: 'none', endpoint_ref: null, observed_at: null, evidence_digest: null })) {
      add('$.evidence.independent_verification', 'unpublished state must remain not_run with null evidence')
    }
  }
  if (typeof evidence.package_id === 'string' && typeof evidence.type_tag === 'string' &&
      !evidence.type_tag.startsWith(`${evidence.package_id}::`)) {
    add('$.evidence.type_tag', 'package prefix must equal package_id')
  }

  const gateNames = ['legal', 'security_audit', 'multisig', 'recovery', 'migration', 'treasury_reconciliation']
  const gates = candidate.mainnet_gates || {}
  const everyGateApproved = gateNames.every((name) => gates[name]?.status === 'approved' && typeof gates[name]?.evidence_ref === 'string')
  if (gates.all_approved !== everyGateApproved) add('$.mainnet_gates.all_approved', 'must equal the aggregate gate decision')
  if (mainnetControlledStates.has(state) && !everyGateApproved) add('$.mainnet_gates', 'mainnet ready/published requires every gate and evidence')
  if (mainnetControlledStates.has(state) && candidate.upgrade_policy === 'pending') {
    add('$.upgrade_policy', 'must be decided before mainnet readiness')
  }

  const secretKeyPattern = /(private[_-]?key|mnemonic|seed[_-]?phrase|password|passphrase|api[_-]?secret|client[_-]?secret|access[_-]?token|refresh[_-]?token|bearer[_-]?token|withdrawal[_-]?key|rpc[_-]?api[_-]?key)/i
  const secretValuePattern = /(-----BEGIN [A-Z ]*PRIVATE KEY-----|\b(?:private[_ -]?key|mnemonic|seed[_ -]?phrase|password|passphrase|api[_ -]?secret|client[_ -]?secret|access[_ -]?token|refresh[_ -]?token|withdrawal[_ -]?key|rpc[_ -]?api[_ -]?key)\s*[:=]\s*\S+|\bbearer\s+[A-Za-z0-9._~+\/-]{8,}|\bsuiprivkey1[0-9a-z]{20,}|\bgithub_pat_[A-Za-z0-9_]{20,}|\bgh[pousr]_[A-Za-z0-9]{20,}|\bsk-[A-Za-z0-9]{20,})/i
  const scan = (value, location = '$') => {
    if (Array.isArray(value)) return value.forEach((item, index) => scan(item, `${location}[${index}]`))
    if (value && typeof value === 'object') {
      for (const [key, child] of Object.entries(value)) {
        if (secretKeyPattern.test(key)) add(`${location}.${key}`, 'secret-shaped field name is forbidden')
        scan(child, `${location}.${key}`)
      }
    } else if (typeof value === 'string' && secretValuePattern.test(value)) {
      add(location, 'secret-shaped string value is forbidden')
    }
  }
  scan(candidate)
  for (const [name, content] of Object.entries({ moveToml, moveSource, moveTests, requirement, decision })) {
    if (secretValuePattern.test(content)) add(`$.repository.${name}`, 'secret-shaped raw source content is forbidden')
  }
  if (typeof candidate.asset?.icon_url === 'string') {
    try {
      const icon = new URL(candidate.asset.icon_url)
      if (!['https:', 'ipfs:'].includes(icon.protocol) || icon.username || icon.password || icon.search || icon.hash) {
        add('$.asset.icon_url', 'must be public https/ipfs without credentials, query, or fragment')
      }
    } catch {
      add('$.asset.icon_url', 'must be a valid public https/ipfs URL')
    }
  }
  return errors
}

function allErrors(candidate, predecessor = null) {
  return [...validate(schema, candidate), ...semanticErrors(candidate, predecessor)]
}

function assertInvalid(candidate, expectedFragment) {
  const errors = allErrors(candidate)
  assert.ok(errors.some((error) => error.includes(expectedFragment)), `expected failure containing ${expectedFragment}; got ${errors.join('; ')}`)
}

function stripMoveComments(source) {
  return source.replace(/\/\*[\s\S]*?\*\//g, '').replace(/\/\/.*$/gm, '')
}

const manifestErrors = allErrors(manifest, predecessorContext)
assert.deepEqual(manifestErrors, [], `genesis manifest must satisfy schema and semantics:\n${manifestErrors.join('\n')}`)

if (isDefaultManifest) {
  assert.equal(manifest.lifecycle.state, 'local_verified')
  assert.equal(manifest.lifecycle.revision, 1)
  assert.equal(manifest.toolchain.sui_release, 'testnet-v1.79.0')
  assert.equal(manifest.toolchain.sui_source_commit, '46f18562f1f5af2438d35828e8b62d5e0b972db7')
  assert.ok(Object.values(manifest.roles).every((role) => role.holder_ref === null), 'local fixture must not invent chain holders')
}

if (isDefaultManifest) {
  const promisedPeg = clone(manifest)
  promisedPeg.assurances.permanent_usdt_peg = true
  assertInvalid(promisedPeg, '$.assurances.permanent_usdt_peg')

const duplicateBucket = clone(manifest)
duplicateBucket.allocations[1].bucket_id = duplicateBucket.allocations[0].bucket_id
assertInvalid(duplicateBucket, '$.allocations.bucket_id')

const wrongSupplyTotal = clone(manifest)
wrongSupplyTotal.allocations[0].base_units = (BigInt(wrongSupplyTotal.allocations[0].base_units) + 1n).toString()
assertInvalid(wrongSupplyTotal, '$.allocations.base_units')

const wrongProportion = clone(manifest)
wrongProportion.allocations[0].base_units = (BigInt(wrongProportion.allocations[0].base_units) + 1n).toString()
wrongProportion.allocations[1].base_units = (BigInt(wrongProportion.allocations[1].base_units) - 1n).toString()
assertInvalid(wrongProportion, 'allocation proportion')

const duplicateHolder = clone(manifest)
duplicateHolder.roles.treasury.holder_ref = `sui:0x${'1'.repeat(64)}`
duplicateHolder.roles.upgrade.holder_ref = duplicateHolder.roles.treasury.holder_ref
assertInvalid(duplicateHolder, '$.roles.holder_ref')

const weakCustody = clone(manifest)
weakCustody.roles.team_vesting.custody = 'service_principal'
assertInvalid(weakCustody, '$.roles.team_vesting')

  const prematureEvidence = clone(manifest)
  prematureEvidence.evidence.allocation_tx_digest = '2'.repeat(32)
  assertInvalid(prematureEvidence, '$.evidence.allocation_tx_digest')

  const prematurePublication = clone(manifest)
  prematurePublication.lifecycle.state = 'testnet_published'
  assertInvalid(prematurePublication, '$.evidence.registered_currency_object_id')
  assertInvalid(prematurePublication, 'online chain verifier')

  const redirectedAllocation = clone(prematurePublication)
  operationalRoles.forEach((roleName, index) => { redirectedAllocation.roles[roleName].holder_ref = `sui:0x${String(index + 1).repeat(64)}` })
  redirectedAllocation.evidence.allocation_tx_digest = '2'.repeat(32)
  redirectedAllocation.evidence.allocation_receipts = redirectedAllocation.allocations.map((item, index) => ({ bucket_id: item.bucket_id, base_units: item.base_units, recipient_role: item.recipient_role, recipient_holder_ref: redirectedAllocation.roles[item.recipient_role].holder_ref, vesting_policy_ref: item.vesting_policy_ref, enforcement: item.bucket_id === 'team_vesting' ? 'move_contract' : 'approved_custody', resulting_object_id: `0x${(index + 1).toString(16).repeat(64)}`, tx_digest: redirectedAllocation.evidence.allocation_tx_digest, checkpoint: '1' }))
  redirectedAllocation.evidence.allocation_receipts[0].recipient_role = 'treasury'
  assertInvalid(redirectedAllocation, '$.evidence.allocation_receipts.user_migration_and_ecosystem')

  const brokenPredecessor = clone(manifest)
  brokenPredecessor.lifecycle.revision = 3
  brokenPredecessor.lifecycle.predecessor_manifest_id = manifest.manifest_id
  brokenPredecessor.lifecycle.predecessor_revision = 1
  brokenPredecessor.lifecycle.predecessor_digest = `sha256:${'3'.repeat(64)}`
  assertInvalid(brokenPredecessor, '$.lifecycle.predecessor_revision')

  const sameStateSuccessor = clone(manifest)
  sameStateSuccessor.lifecycle.revision = 2
  sameStateSuccessor.lifecycle.predecessor_manifest_id = manifest.manifest_id
  sameStateSuccessor.lifecycle.predecessor_revision = 1
  sameStateSuccessor.lifecycle.predecessor_digest = canonicalTextSha256(fs.readFileSync(manifestFile))
  assert.ok(allErrors(sameStateSuccessor, { manifest, digest: sameStateSuccessor.lifecycle.predecessor_digest }).some((error) => error.includes('next legal state')))

const wrongNetworkState = clone(manifest)
wrongNetworkState.lifecycle.state = 'mainnet_ready'
assertInvalid(wrongNetworkState, '$.lifecycle.state')

const unsupportedMainnet = clone(manifest)
unsupportedMainnet.chain.network = 'mainnet'
unsupportedMainnet.toolchain.sui_release = 'mainnet-v1.79.0'
unsupportedMainnet.roles.deployer.custody = 'multisig_required'
unsupportedMainnet.roles.deployer.minimum_approvals = 2
assertInvalid(unsupportedMainnet, '$.chain.network')

const wrongReleaseChannel = clone(manifest)
wrongReleaseChannel.toolchain.sui_release = 'mainnet-v1.79.0'
assertInvalid(wrongReleaseChannel, '$.toolchain.sui_release')

const falsePlannedEvidence = clone(manifest)
falsePlannedEvidence.lifecycle.state = 'planned'
assertInvalid(falsePlannedEvidence, '$.toolchain.verification_status')

const wrongPinnedCommit = clone(manifest)
wrongPinnedCommit.toolchain.sui_source_commit = '0'.repeat(40)
assertInvalid(wrongPinnedCommit, 'Move.toml pinned Sui revision')

const staleSourceDigest = clone(manifest)
staleSourceDigest.toolchain.move_source_digest = `sha256:${'0'.repeat(64)}`
assertInvalid(staleSourceDigest, '$.toolchain.move_source_digest')

const stalePackageDigest = clone(manifest)
stalePackageDigest.toolchain.package_input_digest = `sha256:${'0'.repeat(64)}`
assertInvalid(stalePackageDigest, '$.toolchain.package_input_digest')

const invalidCalendarDate = clone(manifest)
invalidCalendarDate.lifecycle.updated_at = '2026-02-30T12:00:00Z'
assertInvalid(invalidCalendarDate, '$.lifecycle.updated_at')

const invalidClaims = clone(manifest)
invalidClaims.paid_user_migration.claims_enabled = true
assertInvalid(invalidClaims, '$.paid_user_migration.claims_enabled')

const missingClaimField = clone(manifest)
missingClaimField.paid_user_migration.required_claim_fields.pop()
assertInvalid(missingClaimField, '$.paid_user_migration.required_claim_fields')

const secretValue = clone(manifest)
secretValue.asset.description = 'Bearer abcdefghijklmnopqrstuvwxyz'
assertInvalid(secretValue, '$.asset.description')

const passwordValue = clone(manifest)
passwordValue.asset.description = 'Public description password=correct-horse-battery-staple'
assertInvalid(passwordValue, '$.asset.description')

const suiPrivateKey = clone(manifest)
suiPrivateKey.asset.description = `Public description suiprivkey1${'a'.repeat(48)}`
assertInvalid(suiPrivateKey, '$.asset.description')

const unsafeIcon = clone(manifest)
unsafeIcon.asset.icon_url = 'https://example.test/icon.png?api_key=secret'
assertInvalid(unsafeIcon, '$.asset.icon_url')

  const mismatchedTypeTag = clone(manifest)
  mismatchedTypeTag.evidence.package_id = `0x${'1'.repeat(64)}`
  mismatchedTypeTag.evidence.type_tag = `0x${'2'.repeat(64)}::esk::ESK`
  assertInvalid(mismatchedTypeTag, '$.evidence.type_tag')
}

const code = stripMoveComments(moveSource)
assert.match(moveToml, /name\s*=\s*"esk_currency"/)
assert.match(moveToml, /edition\s*=\s*"2024"/)
assert.ok(suiDependencyBlock, 'Move.toml must declare the Sui dependency inline')
assert.match(suiDependencyBlock, /git\s*=\s*"https:\/\/github\.com\/MystenLabs\/sui\.git"/)
assert.match(suiDependencyBlock, /rev\s*=\s*"[0-9a-f]{40}"/)
assert.doesNotMatch(suiDependencyBlock, /rev\s*=\s*"(?:main|testnet|mainnet)"/)
assert.match(code, /module\s+esk_currency::esk;/)
assert.match(code, /public struct ESK has drop \{\}/)
assert.match(code, /coin_registry::new_currency_with_otw\(/)
assert.equal((code.match(/\.mint\(/g) || []).length, 1, 'currency init must mint exactly once')
assert.match(code, /currency\.make_supply_fixed\(treasury_cap\)/)
assert.doesNotMatch(code, /make_supply_burn_only|make_regulated|DenyCap|public fun mint/)
assert.doesNotMatch(code, /public_transfer\(treasury_cap/)
assert.match(code, /public_transfer\(metadata_cap, ctx\.sender\(\)\)/)
assert.match(code, /public_transfer\(total_supply, ctx\.sender\(\)\)/)

const moveSupply = code.match(/const TOTAL_SUPPLY_BASE_UNITS: u64 = ([0-9_]+);/)
const moveDecimals = code.match(/const DECIMALS: u8 = ([0-9]+);/)
assert.ok(moveSupply, 'Move supply constant is missing')
assert.ok(moveDecimals, 'Move decimals constant is missing')
assert.equal(BigInt(moveSupply[1].replaceAll('_', '')), BigInt(manifest.supply.base_units))
assert.equal(Number(moveDecimals[1]), manifest.asset.decimals)
assert.match(moveTests, /decimals_match_genesis_manifest/)
assert.match(moveTests, /supply_matches_genesis_manifest/)
assert.match(moveTests, /initializer_mints_once_and_consumes_treasury_cap/)
assert.match(moveTests, /has_most_recent_for_sender<TreasuryCap<ESK>>/)

for (const marker of ['finalize_registration', 'CoinRegistry 0xc', 'QSHARE', 'Sponsor Junior Capital']) {
  assert.ok(`${requirement}\n${decision}`.includes(marker), `missing architecture or release marker: ${marker}`)
}
for (const forbidden of ['task_sui_projection', 'task_sui_preflight']) {
  assert.equal(moveSource.includes(forbidden), false, `ESK currency must not reuse task settlement domain: ${forbidden}`)
}

console.log('ESK_SUI_GENESIS_SCHEMA_AND_SEMANTIC_TEST=passed')
console.log('ESK_SUI_SUPPLY_CONSERVATION_TEST=passed')
console.log('ESK_SUI_MOVE_SOURCE_BINDING_TEST=passed')
console.log(`ESK_SUI_MANIFEST_DECLARED_STATE=${manifest.lifecycle.state}`)
console.log('ESK_SUI_MOVE_RUNTIME_EXECUTION=not_performed_by_this_script')
