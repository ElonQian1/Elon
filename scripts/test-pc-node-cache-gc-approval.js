const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const routes = source('server/src/router/node_routes.rs')
const api = source('server/src/node_api/rust_cache_fleet/gc.rs')
const contract = source('server/src/node_api/rust_cache_fleet/gc_contract.rs')
const nodeRuntime = source('server/src/node_agent_rust_cache_fleet/gc.rs')
const model = source('pc-frontend/src/features/node/nodeCacheGc.ts')
const approval = source('pc-frontend/src/features/node/NodeCacheGcApproval.tsx')
const cacheEntry = source('scripts/rust-cache.ps1')
const localApproval = source('scripts/rust-cache/RustCache.GcApproval.psm1')

for (const route of [
  '/api/me/nodes/:node_id/cache-gc',
  '/api/me/nodes/:node_id/cache-gc/:request_id/approve',
  '/api/me/nodes/:node_id/cache-gc/:request_id/reject',
  '/api/node/cache-gc/:node_id/next',
  '/api/node/cache-gc/:node_id/:request_id/plan',
  '/api/node/cache-gc/:node_id/:request_id/result',
]) assert.match(routes, new RegExp(escapeRegExp(route)))

assert.match(api, /APPROVE_EXACT_GC_PLAN/)
assert.match(api, /server_has_absolute_paths": false/)
assert.match(contract, /destructive_actions_authorized/)
assert.match(contract, /target_rescan_required/)
assert.match(nodeRuntime, /platform"\)\.join\("rust-cache\.ps1"\)/)
assert.match(nodeRuntime, /WindowStyle"\)\.arg\("Hidden"\)/)
assert.match(nodeRuntime, /CREATE_NO_WINDOW|0x0800_0000/)
assert.doesNotMatch(nodeRuntime, /cmd\.exe|Start-Process/)
assert.match(model, /acknowledge_remote_gc: true/)
assert.match(model, /APPROVE_EXACT_GC_PLAN/)
assert.match(model, /server_has_absolute_paths !== false/)
assert.match(model, /absolute_paths_included/)
assert.match(approval, /window\.confirm/)
assert.match(approval, /目标电脑会重新扫描/)
assert.doesNotMatch(`${model}\n${approval}`, /cache_root|repo_root|workspace_path/i)
assert.match(cacheEntry, /"gc-plan"/)
assert.match(cacheEntry, /"gc-apply-approved"/)
assert.match(localApproval, /exact_action_set = \$true/)
assert.match(localApproval, /active_writer_count_unchanged = \$true/)
assert.match(localApproval, /local_rescan_completed = \$true/)

console.log('PC node cache GC approval contracts passed')

function source(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), 'utf8')
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}
