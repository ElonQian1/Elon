#!/usr/bin/env node

const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const read = (relative) => fs.readFileSync(path.join(root, relative), 'utf8')
const projectionSchema = JSON.parse(read('contracts/quant/esk-paper-asset-projection-v1.schema.json'))
const projectionSchemaV2 = JSON.parse(read('contracts/quant/esk-paper-asset-projection-v2.schema.json'))
const launchSchema = JSON.parse(read('contracts/quant/paper-launch-v1.schema.json'))
const issuer = read('server/src/quant_esk_asset_projection.rs')
const launch = read('server/src/quant_paper_launch.rs')
const bridge = read('pc-frontend/src/features/conversation/QuantPaperLaunch.tsx')
const catalog = JSON.parse(read('server/src/official_project_catalog/catalog.json'))

assert.equal(projectionSchema.additionalProperties, false)
assert.equal(projectionSchema.properties.schema.const, 'yilong.esk.asset_projection.v1')
assert.equal(projectionSchema.properties.asset_id.const, 'esk')
assert.equal(projectionSchema.properties.symbol.const, 'ESK')
assert.equal(projectionSchema.properties.name.const, '一龙 ESK')
assert.equal(projectionSchema.properties.chain_status.const, 'not_deployed')
assert.equal(projectionSchema.properties.simulated.const, true)
assert.equal(projectionSchema.properties.funds_moved.const, false)
assert.equal(projectionSchemaV2.additionalProperties, false)
assert.equal(projectionSchemaV2.properties.schema.const, 'yilong.esk.asset_projection.v2')
for (const field of ['reserved_for_quant', 'reserved_total', 'quant_reserved_base_units']) {
  assert.ok(projectionSchemaV2.required.includes(field), `projection v2 must require ${field}`)
}
for (const field of [
  'grant_id',
  'participant_ref',
  'total_base_units',
  'available_base_units',
  'reserved_base_units',
  'source_revision',
  'observed_at_unix',
  'expires_at_unix',
]) assert.ok(projectionSchema.required.includes(field), `projection schema must require ${field}`)

assert.match(launchSchema.$defs.launchTicket.properties.esk_asset_projection.pattern, /^\^yep\[12\]/)
assert.match(launchSchema.$defs.grantMessage.properties.esk_asset_projection.pattern, /^\^yep\[12\]/)
assert.deepEqual(
  launchSchema.$defs.readyMessage.properties.capabilities.items.enum,
  ['yilong.esk.asset_projection.v1', 'yilong.esk.asset_projection.v2'],
)

for (const required of [
  'issue_esk_projection',
  'issue_esk_projection_v2',
  'ledger.reserved_base_units > ledger.total_base_units',
  'funds_moved: false',
  'chain_status: "not_deployed"',
  'signer.sign_token(TOKEN_PREFIX_V2, &claims)',
]) assert.ok(issuer.includes(required), `issuer missing ${required}`)
assert.ok(launch.includes('state.store.esk_account_ledger(&user_id)'))
assert.ok(launch.includes('&grant.grant_id'))
assert.ok(launch.includes('&grant.participant_ref'))
assert.ok(launch.includes('validate_capabilities'))

assert.ok(bridge.includes('capabilities: [...ESK_PROJECTION_SCHEMAS]'))
assert.ok(bridge.includes('supportsEskProjection'))
assert.ok(bridge.includes('grantMessage.esk_asset_projection'))
for (const forbidden of ['localStorage', 'sessionStorage', 'indexedDB', 'location.search', 'targetOrigin: \'*\'']) {
  assert.equal(bridge.includes(forbidden), false, `bridge contains forbidden ${forbidden}`)
}

const quantLanding = catalog.projects.find((project) => project.id === 'yilong-quant')?.landing
assert.ok(quantLanding, 'official catalog must contain yilong-quant landing')
assert.ok(quantLanding.summary.includes('ESK Paper 余额的签名只读显示'))
assert.ok(quantLanding.highlights.some((item) => item.includes('主项目 ESK 余额')))
assert.ok(quantLanding.highlights.some((item) => item.includes('不会自动改名或折算为 ESK')))
for (const client of ['web', 'windows', 'android']) {
  assert.equal(quantLanding.downloads[client].status, 'planned')
}

console.log('Quant ESK asset projection contract checks passed.')
