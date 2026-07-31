const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const feature = 'pc-frontend/src/features/open-commerce/erp-blueprint'
const shell = read('pc-frontend/src/features/open-commerce/OpenCommercePanel.tsx')
const panel = read(`${feature}/ErpBlueprintPanel.tsx`)
const instance = read(`${feature}/ErpInstanceView.tsx`)
const maintainer = read(`${feature}/BlueprintMaintainerView.tsx`)
const mcp = read('server/src/erp_blueprint_mcp_tools.rs')

assert.ok(shell.includes("id: 'erp'"), 'open commerce workspace must expose the ERP blueprint tab')
assert.ok(shell.includes('ErpBlueprintPanel'), 'ERP blueprint panel must be routed')
assert.ok(panel.includes('同一套稳定内核'), 'panel must explain shared core and independent projects')
assert.ok(instance.includes('resolveRequirement'), 'merchant workspace must resolve requirements before development')
assert.ok(instance.includes('merchant_authorized: true'), 'feature signals require explicit merchant authorization')
assert.ok(instance.includes('private_extensions'), 'upgrade UI must surface protected private extensions')
assert.ok(maintainer.includes('create_matter: true'), 'accepted proposals must enter the existing Matter flow')
assert.ok(maintainer.includes('support_count < blueprint.definition.proposal_threshold'), 'UI must enforce the independent-merchant threshold')

for (const forbidden of ['erp_accept_proposal', 'erp_adopt_upgrade', 'erp_publish_version']) {
  assert.ok(!mcp.includes(`"${forbidden}"`), `${forbidden} must not be exposed to AI agents`)
}
assert.ok(mcp.includes('erp_prepare_upgrade_check'), 'AI agents may prepare a non-executing compatibility check')

console.log('ERP blueprint PC workspace contracts passed')

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), 'utf8')
}
