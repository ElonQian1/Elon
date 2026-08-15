const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const featureRoot = 'pc-frontend/src/features/open-commerce'
const adoptions = read(`${featureRoot}/ConsumerPortabilityAdoptions.tsx`)
const mergePanel = read(`${featureRoot}/ConsumerPortabilityMergePanel.tsx`)
const clientApi = read(`${featureRoot}/openCommerceClientApi.ts`)
const clientTypes = read(`${featureRoot}/openCommerceClientTypes.ts`)

assert.ok(
  adoptions.includes('<ConsumerPortabilityMergePanel'),
  'the portability workspace must mount the multi-source merge panel',
)
assert.ok(
  mergePanel.includes('item.preference_profile_included'),
  'only imports containing a preference profile may be selected',
)
assert.ok(
  mergePanel.includes('current.length < 10'),
  'the client must cap merge plans at ten sources',
)
assert.ok(
  mergePanel.includes('selectedImportIds.length < 2'),
  'the client must require at least two sources',
)
assert.ok(
  mergePanel.includes('setPlan(null)') && mergePanel.includes('setSelections({})'),
  'changing sources must invalidate the prior plan and selections',
)
assert.ok(
  mergePanel.includes(".map(([field, import_id]) => ({ field, import_id }))"),
  'each selected field must carry an explicit source import',
)
assert.ok(
  mergePanel.includes('window.confirm') && mergePanel.includes('每个字段的来源将被记录'),
  'apply must require explicit confirmation and explain provenance recording',
)
assert.ok(
  mergePanel.includes('plan.sources.map((source) => source.import_id)')
    && mergePanel.includes('plan.current_profile_revision'),
  'apply must bind the reviewed sources and current revision',
)
assert.ok(
  mergePanel.includes('adoption.resulting_revision'),
  'rollback must bind the expected resulting revision',
)
assert.ok(
  mergePanel.includes('关系、订单和 ERP 数据保持隔离'),
  'the UI must state that relationships, orders, and ERP data stay isolated',
)

assert.ok(
  clientApi.includes('/consumer-portability-merge-plan`')
    && clientApi.includes('/consumer-portability-merge-adoptions`'),
  'the client must use the dedicated plan and adoption routes',
)
assert.ok(
  clientApi.includes('/consumer-portability-merge-adoptions/${encodeURIComponent(adoptionId)}/rollback`'),
  'rollback identifiers must be encoded in the route',
)
assert.ok(
  clientApi.match(/confirmed_by_user: true/g)?.length >= 2,
  'merge apply and rollback must carry explicit user confirmation',
)

assert.ok(
  clientTypes.includes("schema: 'open_commerce.consumer_portability_merge_plan.v1'"),
  'the plan contract must remain versioned',
)
assert.ok(
  clientTypes.includes('current_profile_revision: number | null')
    && clientTypes.includes('sources: ConsumerPortabilityMergeSource[]')
    && clientTypes.includes('fields: ConsumerPortabilityMergeField[]'),
  'the plan must carry its revision, sources, and field candidates',
)
assert.ok(
  clientTypes.includes('automatic_conflict_resolution: false')
    && clientTypes.includes('automatic_relationship_restore: false')
    && clientTypes.includes('automatic_business_write: false'),
  'the plan contract must keep all automatic side effects disabled',
)
assert.ok(
  clientTypes.includes('source_import_ids: string[]')
    && clientTypes.includes('field_sources: ConsumerPortabilityFieldSource[]')
    && clientTypes.includes('resulting_revision: number'),
  'the adoption receipt must preserve source and revision provenance',
)

console.log('Open commerce portability merge PC contracts passed')

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), 'utf8')
}
