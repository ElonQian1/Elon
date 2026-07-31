const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const examples = path.join(root, 'examples/erp-blueprints')
const blueprint = json('official-blueprint.json')
const release = json('release-1.1.0.json')
const coffee = json('cofficethinking-instance.json')
const retail = json('minimal-retail-instance.json')
const signal = json('feature-signal.json')

assert.equal(blueprint.schema, 'yilong.erp.blueprint.v1')
assert.equal(release.schema, 'yilong.erp.release.v1')
assert.equal(coffee.blueprint_key, blueprint.blueprint_key)
assert.equal(retail.blueprint_key, blueprint.blueprint_key)
assert.equal(coffee.pinned_version, retail.pinned_version)
assert.notEqual(coffee.theme_key, retail.theme_key)
assert.notDeepEqual(coffee.plugins, retail.plugins)
assert.ok(coffee.private_extensions.length > 0)
assert.equal(coffee.data_policy.raw_data_exported_to_blueprint, false)
assert.equal(retail.data_policy.raw_data_exported_to_blueprint, false)
assert.equal(signal.schema, 'yilong.erp.feature_signal.v1')
assert.equal(signal.merchant_authorized, true)
assert.equal(signal.classification, 'sanitized_aggregate')

const extensionPoints = new Set(release.extension_points)
for (const instance of [coffee, retail]) {
  for (const extension of [...instance.plugins, ...instance.private_extensions]) {
    assert.ok(extensionPoints.has(extension.extension_point), `${extension.extension_key} extension point must survive the upgrade`)
  }
}

const beforePrivate = JSON.stringify(coffee.private_extensions)
const afterPrivate = JSON.stringify(structuredClone(coffee).private_extensions)
assert.equal(afterPrivate, beforePrivate, 'public blueprint upgrades must not rewrite private extensions')

for (const schema of [
  'erp-blueprint-v1.schema.json',
  'erp-instance-v1.schema.json',
  'erp-feature-signal-v1.schema.json',
  'erp-release-manifest-v1.schema.json',
]) {
  assert.doesNotThrow(() => JSON.parse(fs.readFileSync(path.join(root, 'contracts/erp', schema), 'utf8')))
}

console.log('ERP blueprint machine contracts passed')

function json(name) {
  return JSON.parse(fs.readFileSync(path.join(examples, name), 'utf8'))
}
